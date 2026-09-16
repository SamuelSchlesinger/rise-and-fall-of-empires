#!/usr/bin/env python3
"""Check the terminal ABI against a real pty on Linux or macOS.

Verifies raw mode, window-size changes, delayed pastes, immediate keyboard
input, and exact restoration after normal exit, SIGHUP, SIGINT and SIGTERM. Uses only
the Python standard library and keeps config/data in a temporary directory.
"""

import argparse
import fcntl
import os
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time


def resize(fd, rows, cols):
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', rows, cols, 0, 0))


def read_until(fd, output, predicate, description):
    deadline = time.monotonic() + 10
    while not predicate(output):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise AssertionError('timed out waiting for ' + description)
        readable, _, _ = select.select([fd], [], [], min(remaining, 0.1))
        if readable:
            data = os.read(fd, 65536)
            if not data:
                raise AssertionError('terminal closed before ' + description)
            output.extend(data)


def drain_for(fd, output, seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        readable, _, _ = select.select([fd], [], [], 0.05)
        if readable:
            data = os.read(fd, 65536)
            if not data:
                return
            output.extend(data)


def check(binary, env, sig):
    master, slave = pty.openpty()
    proc = None
    try:
        initial = termios.tcgetattr(slave)
        input_flags = (termios.BRKINT | termios.ICRNL | termios.INPCK
                       | termios.ISTRIP | termios.IXON)
        local_flags = termios.ECHO | termios.ICANON | termios.IEXTEN | termios.ISIG
        initial[0] |= input_flags
        initial[1] |= termios.OPOST
        initial[3] |= local_flags
        # Non-default values catch wrong control-character offsets and partial
        # restoration even on systems whose default VMIN/VTIME are already 0.
        initial[6][termios.VMIN] = b'\x07'
        initial[6][termios.VTIME] = b'\x03'
        termios.tcsetattr(slave, termios.TCSANOW, initial)
        initial = termios.tcgetattr(slave)
        resize(slave, 37, 113)
        proc = subprocess.Popen(
            [binary, '--seed', '7', '--width', '40', '--height', '20', '--ascii'],
            stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True,
        )
        output = bytearray()
        read_until(master, output, lambda b: b'\x1b[37;1H' in b, 'first frame')
        assert b'\x1b[?1049h' in output, 'did not enter alternate screen'
        raw = termios.tcgetattr(slave)
        assert raw[0] & input_flags == 0, 'input processing is still enabled'
        assert raw[1] & termios.OPOST == 0, 'output processing is still enabled'
        assert raw[3] & local_flags == 0, 'canonical input/echo/signals still enabled'
        assert raw[6][termios.VMIN] == 0, 'VMIN was not cleared'
        assert raw[6][termios.VTIME] == 0, 'VTIME was not cleared'

        resize(slave, 43, 127)
        read_until(master, output, lambda b: b'\x1b[43;1H' in b, 'resized frame')

        if sig is None:
            # A stalled paste must remain text, even when it looks like a
            # command. Previously a 250 ms gap ended paste mode prematurely.
            os.write(master, b'\x1b[200~')
            drain_for(master, output, 0.45)
            os.write(master, b':q!\r')
            drain_for(master, output, 0.45)
            assert proc.poll() is None, 'delayed paste was executed as a command'
            os.write(master, b'\x1b[201~')
            drain_for(master, output, 0.1)
            # ZQ needs no newline: this also exercises poll/read in raw mode.
            os.write(master, b'ZQ')
        else:
            proc.send_signal(sig)
        read_until(master, output, lambda b: b'\x1b[?1049l' in b, 'terminal reset')
        status = proc.wait(timeout=5)
        expected = 0 if sig is None else -sig
        assert status == expected, 'exit status %s, expected %s' % (status, expected)
        assert termios.tcgetattr(slave) == initial, 'terminal settings not restored'
        assert b'panicked' not in output, 'the game panicked'
    finally:
        if proc is not None and proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', nargs='?', default=os.path.join(
        root, 'target', 'release', 'empires'))
    args = parser.parse_args()
    binary = os.path.abspath(args.binary)
    with tempfile.TemporaryDirectory(prefix='termtest-empires-') as tempdir:
        env = dict(os.environ)
        env['TERM'] = 'xterm-256color'
        env['XDG_CONFIG_HOME'] = os.path.join(tempdir, 'config')
        env['XDG_DATA_HOME'] = os.path.join(tempdir, 'data')
        # An existing data directory suppresses the first-run tour, so ZQ
        # reaches the normal input handler without writing a user marker.
        os.makedirs(os.path.join(env['XDG_DATA_HOME'], 'empires'))
        for sig in (None, signal.SIGHUP, signal.SIGINT, signal.SIGTERM):
            label = 'normal exit' if sig is None else signal.Signals(sig).name
            try:
                check(binary, env, sig)
            except (AssertionError, OSError, subprocess.TimeoutExpired) as error:
                print('FAIL: %s: %s' % (label, error))
                return 1
            print('OK: raw mode, resize, input/exit, restoration (%s)' % label)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
