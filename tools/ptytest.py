import os, pty, time, select, struct, fcntl, termios, sys, signal
binary = sys.argv[1]
pid, fd = pty.fork()
if pid == 0:
    os.execv(binary, [binary, '--seed', '2', '--detail', 'high'])
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 45, 160, 0, 0))
out = bytearray()
def pump(t):
    end = time.time() + t
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
pump(1.5)
keys = [b'+', b'5', b'j', b'1', b'0', b'l', b'\x1b[1;2A', b'\x1b[1;5C', b'H', b'J', b'0', b'$', b'z', b'z', b'g', b'g', b'z', b'o', b'z', b'o', b'z', b'i', b't',
        b']', b']', b'[', b'}', b'{', b'/', b'a', b'\r', b'n', b'N', b':', b's', b'p', b'e', b'e', b'd', b' ', b'2', b'5', b'\r',
        b':', b'l', b'a', b'y', b'e', b'r', b' ', b'c', b'u', b'l', b'\r', b':', b's', b't', b'e', b'p', b' ', b'3', b'0', b'\r',
        b':', b't', b'h', b'e', b'm', b'e', b'\r', b':', b't', b'h', b'e', b'm', b'e', b' ', b'p', b'a', b'p', b'e', b'r', b'\r', b':', b's', b'e', b't', b' ', b'z', b'o', b'o', b'm', b' ', b'3', b'\r',
        b':', b'm', b'a', b'p', b' ', b'w', b' ', b'k', b'\r', b'w', b':', b'm', b'u', b't', b'e', b' ', b'b', b'a', b't', b't', b'l', b'e', b'\r', b':', b's', b't', b'o', b'r', b'y', b'\r',
        b':', b'w', b' ', b'/', b't', b'm', b'p', b'/', b'p', b't', b'y', b't', b'e', b's', b't', b'.', b'r', b'f', b'e', b'\r', b'.', b'.', b':', b'e', b' ', b'/', b't', b'm', b'p', b'/', b'p', b't', b'y', b't', b'e', b's', b't', b'.', b'r', b'f', b'e', b'\r',
        b'3', b'.', b' ', b'e', b'\t', b'\t', b'\t', b'\t', b'\t', b'\t', b'\t', b'/', b'a', b'\r', b'j', b'\r', b'\x04', b'\x15', b'G', b'g', b'g', b'm',
        b'c', b'/', b'w', b'a', b'r', b'\r', b'k', b'k', b'\x1b', b'\x1b', b'?', b'\x1b',
        b'\x1b[<0;40;10M', b'\x1b[<0;40;10m', b'\x1b[<0;40;10M', b'\x1b[<0;40;10m', b'\x1b', b'\x1b[<64;40;10M', b'\x1b[<65;40;10M',
        b'\x1b[<0;150;44M', b'\x1b[<0;150;44m', b'\x1b', b'x', b'\x1b', b'\x1b[<2;40;10M', b'\x1b[<2;40;10m', b'\x1b',
        b':', b'n', b'e', b'w', b' ', b'5', b'\r', b'D', b'v', b'\t', b'\x1b[Z', b'x', b'1', b':', b'f', b'i', b'n', b'd', b' ', b'z', b'z', b'z', b'z', b'\r',
        b'Z', b'x', b'q', b'q']
for k in keys:
    os.write(fd, k)
    if not pump(0.15):
        break
pump(2.0)
os.write(fd, b'q')
alive = pump(1.0)
try:
    _, status = os.waitpid(pid, os.WNOHANG)
except ChildProcessError:
    status = 0
if alive:
    time.sleep(0.5)
    try:
        _, status = os.waitpid(pid, os.WNOHANG)
    except ChildProcessError:
        status = 0
    if status == 0 and alive:
        # still running?
        try:
            os.kill(pid, 0)
            print("STILL RUNNING after q; killing")
            os.kill(pid, signal.SIGKILL)
            status = -1
        except ProcessLookupError:
            pass
text = out.decode('utf-8', 'replace')
print("bytes:", len(out), "exit status:", status)
print("alt screen enter:", '\x1b[?1049h' in text, " leave:", '\x1b[?1049l' in text)
print("panic:", 'panicked' in text)
if 'panicked' in text:
    i = text.find('panicked'); print(text[i-200:i+600])
