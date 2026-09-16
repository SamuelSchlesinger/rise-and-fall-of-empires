#!/usr/bin/env python3
"""Drive the real interface in a pseudo-terminal.

Starts the game on a pty, sends a few hundred keystrokes covering every mode,
the `:` commands, search, the mouse and a save/load round trip, and then quits
it. Fails (exit 1) on a panic, a non-zero exit status, a process that will not
quit, or a terminal left on the alternate screen.

The game is run inside a throwaway HOME with XDG_CONFIG_HOME, XDG_DATA_HOME and
XDG_STATE_HOME pointing into it, so the first-run marker, the config file and
any saves it writes never touch the caller's own data. The directory is removed
afterwards.

Usage:
    tools/ptytest.py [BINARY] [--seed N] [--keep] [--verbose]

BINARY defaults to target/release/empires, relative to the repository root.
"""

import argparse
import fcntl
import os
import pty
import select
import shutil
import signal
import struct
import sys
import tempfile
import termios
import time

# Each entry is one write to the pty. Keys are grouped by what they exercise so
# that a failure points somewhere; the game is driven forward by time between
# them regardless.
KEYS = [
    # speed, motion, jumps, zoom, layers
    b'+', b'5', b'j', b'1', b'0', b'l', b'\x1b[1;2A', b'\x1b[1;5C',
    b'H', b'J', b'0', b'$', b'z', b'z', b'g', b'g', b'z', b'o', b'z', b'o',
    b'z', b'i', b't', b']', b']', b'[', b'}', b'{',
    # search
    b'/', b'a', b'\r', b'n', b'N',
    # commands
    *b':speed 25\r', *b':layer cul\r', *b':step 30\r',
    *b':theme\r', *b':theme paper\r', *b':set zoom 3\r',
    *b':map w k\r', b'w', *b':mute battle\r', *b':story\r',
    *b':recap 100\r', b'\x1b',
    # SAVE/LOAD placeholder: filled in at run time with the temp directory
    b'@save@',
    # lists, detail pages, scrolling, links
    b'3', b'.', b' ', b'e', b'\t', b'\t', b'\t', b'\t', b'\t', b'\t', b'\t',
    b'/', b'a', b'\r', b'j', b'\r', b'\x04', b'\x15', b'G', b'g', b'g', b'm',
    # the chronicle, help
    b'c', b'/', b'w', b'a', b'r', b'\r', b'k', b'k', b'\x1b', b'\x1b',
    b'?', b'\x1b',
    # the embedded guide and a complete tutorial through the real controls
    b'?', b'p', b'j', b'\x06', b'G', b'g', b'\x1b',
    b'?', b't', b' ', b' ', b'h', b'l', b'\t', b'z', b'o', b'e', b'\x1b', b'\r',
    *b':guide\r', b'\x1b', *b':tutorial\r', b'\x07',
    # the mouse: click, double click, wheel, right click
    b'\x1b[<0;40;10M', b'\x1b[<0;40;10m', b'\x1b[<0;40;10M', b'\x1b[<0;40;10m',
    b'\x1b', b'\x1b[<64;40;10M', b'\x1b[<65;40;10M',
    b'\x1b[<0;150;44M', b'\x1b[<0;150;44m', b'\x1b', b'x', b'\x1b',
    b'\x1b[<2;40;10M', b'\x1b[<2;40;10m', b'\x1b',
    # bracketed paste: must be treated as text, never run as commands
    b'\x1b[200~', b':q!\r', b'\x1b[201~', b'\x1b',
    # a fresh world, detail, filters, the hand of fate, a search that misses
    *b':new 5\r', b'D', b'v', b'\t', b'\x1b[Z', b'x', b'1',
    *b':find zzzz\r',
]

# Sent after KEYS. An interrupted ZZ must not quit; the two q's then should.
# The game is expected to go away somewhere in here, so an exit during these
# is not an early exit.
QUIT_KEYS = [b'Z', b'x', b'q', b'q']


def expand(keys, save_path):
    """Replace the save placeholder with a save/load round trip."""
    out = []
    for k in keys:
        if k == b'@save@':
            out += [bytes([c]) for c in b':w ' + save_path + b'\r']
            out += [b'.', b'.']
            out += [bytes([c]) for c in b':e ' + save_path + b'\r']
        else:
            out.append(bytes([k]) if isinstance(k, int) else k)
    return out


def run(binary, seed, home, verbose):
    """Run the game on a pty. Returns (output, exit status, notes)."""
    env = dict(os.environ)
    env['HOME'] = home
    env['XDG_CONFIG_HOME'] = os.path.join(home, '.config')
    env['XDG_DATA_HOME'] = os.path.join(home, '.local', 'share')
    env['XDG_STATE_HOME'] = os.path.join(home, '.local', 'state')
    env['XDG_CACHE_HOME'] = os.path.join(home, '.cache')
    env['TERM'] = 'xterm-256color'
    for v in ('XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME'):
        os.makedirs(env[v], exist_ok=True)

    pid, fd = pty.fork()
    if pid == 0:
        # Child: nothing here may raise past execve, so failures exit loudly.
        try:
            os.execve(binary, [binary, '--seed', str(seed), '--detail', 'high'], env)
        except Exception as e:  # noqa: BLE001 - the child cannot report any other way
            sys.stderr.write('exec failed: %s\n' % e)
        os._exit(127)

    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 45, 160, 0, 0))
    out = bytearray()

    def pump(seconds):
        """Read for `seconds`. False once the child's side of the pty is gone."""
        end = time.time() + seconds
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 0.05)
            if r:
                try:
                    data = os.read(fd, 65536)
                except OSError:
                    return False
                if not data:
                    return False
                out.extend(data)
        return True

    notes = []
    pump(1.5)
    keys = expand(KEYS, os.path.join(home, 'world.rfe').encode())
    quit_from = len(keys)
    keys += QUIT_KEYS
    for i, k in enumerate(keys):
        try:
            os.write(fd, k)
            gone = not pump(0.15)
        except OSError:
            gone = True
        if gone:
            if i < quit_from:
                notes.append('exited early, at keystroke %d of %d (%r)'
                             % (i, quit_from, k))
            break
    pump(2.0)

    try:
        os.write(fd, b'q')
    except OSError:
        pass
    alive = pump(1.0)

    status = None
    for _ in range(20):  # up to ~2s for the child to be reaped
        try:
            done, st = os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            status = 0
            break
        if done:
            status = st
            break
        time.sleep(0.1)

    if status is None:
        notes.append('still running after q; killed')
        os.kill(pid, signal.SIGKILL)
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
        status = -1
    elif alive and verbose:
        notes.append('the pty stayed open until the end')

    os.close(fd)
    return bytes(out), status, notes


def describe(status):
    """A process status from waitpid, in words."""
    if status == -1:
        return 'killed (would not quit)'
    if os.WIFSIGNALED(status):
        return 'killed by signal %d' % os.WTERMSIG(status)
    if os.WIFEXITED(status):
        return 'exit code %d' % os.WEXITSTATUS(status)
    return 'status %d' % status


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    default = os.path.join(root, 'target', 'release', 'empires')

    ap = argparse.ArgumentParser(
        description=__doc__.split('\n\n')[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='The game runs in a throwaway HOME, so it cannot touch your '
               'config, saves or first-run marker.',
    )
    ap.add_argument('binary', nargs='?', default=default,
                    help='the game binary (default: target/release/empires)')
    ap.add_argument('--seed', type=int, default=2, help='world seed (default: 2)')
    ap.add_argument('--keep', action='store_true',
                    help='keep the throwaway HOME and print its path')
    ap.add_argument('--verbose', action='store_true',
                    help='print the tail of the output on failure')
    args = ap.parse_args()

    if not os.path.isfile(args.binary) or not os.access(args.binary, os.X_OK):
        print('not an executable: %s' % args.binary, file=sys.stderr)
        print('build it first: cargo build --release', file=sys.stderr)
        return 1

    home = tempfile.mkdtemp(prefix='ptytest-empires-')
    try:
        out, status, notes = run(args.binary, args.seed, home, args.verbose)
    finally:
        if args.keep:
            print('kept %s' % home)
        else:
            shutil.rmtree(home, ignore_errors=True)

    text = out.decode('utf-8', 'replace')
    panicked = 'panicked' in text
    entered = '\x1b[?1049h' in text
    left = '\x1b[?1049l' in text

    print('bytes: %d' % len(out))
    print('exit: %s' % describe(status))
    print('alternate screen: entered %s, left %s' % (entered, left))
    print('panic: %s' % panicked)
    for n in notes:
        print('note: %s' % n)

    failures = [n for n in notes if n.startswith('exited early')]
    if panicked:
        i = text.find('panicked')
        print('\n--- panic ---\n%s\n-------------' % text[max(0, i - 200):i + 600])
        failures.append('the game panicked')
    if status != 0:
        failures.append('bad exit: %s' % describe(status))
    if not entered:
        failures.append('never entered the alternate screen')
    if not left:
        failures.append('did not restore the terminal (no alternate-screen exit)')
    if len(out) < 10000:
        failures.append('suspiciously little output (%d bytes)' % len(out))

    if failures:
        if args.verbose:
            print('\n--- last 2000 bytes ---\n%s' % text[-2000:])
        for f in failures:
            print('FAIL: %s' % f, file=sys.stderr)
        return 1

    print('OK')
    return 0


if __name__ == '__main__':
    sys.exit(main())
