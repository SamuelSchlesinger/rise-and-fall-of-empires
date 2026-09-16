//! Minimal raw-mode terminal layer for Linux and macOS using libc FFI (no crates):
//! termios raw mode, window size, polled key input, bracketed paste and a
//! diff-rendered cell buffer emitting 24-bit ANSI colour.
//!
//! # Platform assumptions
//!
//! [`sys`] owns the platform-specific libc layouts, constants and symbols.
//! It rejects unsupported targets so an ABI mismatch cannot silently corrupt
//! the caller's terminal settings. Input decoding and rendering are shared.

mod sys;

use sys::*;

use std::cell::UnsafeCell;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

/// Everything `enter` switched on, switched off again: bracketed paste, mouse
/// reporting, attributes, the cursor and the alternate screen.
const RESET: &[u8] = b"\x1b[?2004l\x1b[?1006l\x1b[?1000l\x1b[0m\x1b[?25h\x1b[2J\x1b[H\x1b[?1049l";

/// The terminal settings as we found them, for the ordinary `leave()` path.
/// Reading a `OnceLock` neither allocates nor blocks, so the panic hook may
/// use it and `leave()` stays correct however often it is called.
static ORIG: OnceLock<Termios> = OnceLock::new();

/// A second, plain copy of the same settings for the signal handler, which
/// may not lock, allocate or run Rust's I/O machinery.
struct SigCell(UnsafeCell<Termios>);

// SAFETY: the cell is written at most once, by whichever call to `enter` wins
// `ORIG_RAW_CLAIM`, and never again. A reader only looks at it once
// `ORIG_RAW_SET` reads true, and that release/acquire pair orders the write
// before every such read, so readers only ever see fully initialised bytes
// and no read ever races with the write.
unsafe impl Sync for SigCell {}

static ORIG_RAW: SigCell = SigCell(UnsafeCell::new(Termios::ZERO));
/// Taken by the one call to `enter` that is allowed to write `ORIG_RAW`.
static ORIG_RAW_CLAIM: AtomicBool = AtomicBool::new(false);
/// Set once `ORIG_RAW` holds real settings; the handler checks it first.
static ORIG_RAW_SET: AtomicBool = AtomicBool::new(false);
/// Set once the termination handlers have been installed.
static HANDLERS: AtomicBool = AtomicBool::new(false);

fn errno() -> i32 {
    // SAFETY: the platform's errno accessor returns a valid, suitably aligned
    // pointer to this thread's `errno`, which lives for as long as the thread.
    unsafe { *errno_location() }
}

/// Put the terminal back using nothing but raw syscalls: no allocation, no
/// locks, no Rust I/O. Async-signal-safe, idempotent, and harmless before
/// `enter` has ever run.
///
/// # Safety
///
/// Writes to fd 1 and calls `tcsetattr` on fd 0, so the caller must be happy
/// for the terminal to be reset.
unsafe fn restore_raw() {
    let mut off = 0usize;
    while off < RESET.len() {
        // SAFETY: `off < RESET.len()`, so `add(off)` stays inside `RESET` and
        // the length passed is exactly the bytes left in it. `write` only
        // reads from that buffer, and `RESET` is a `'static` constant.
        let n = unsafe { write(1, RESET.as_ptr().add(off).cast(), RESET.len() - off) };
        if n > 0 {
            off += n as usize;
        } else if n < 0 && errno() == EINTR {
            continue;
        } else {
            break;
        }
    }
    if ORIG_RAW_SET.load(Ordering::Acquire) {
        // SAFETY: `ORIG_RAW_SET` is only released after `ORIG_RAW` has been
        // written, so the pointer is to an initialised `Termios` that nothing
        // writes again; `tcsetattr` only reads through it.
        unsafe { tcsetattr(0, TCSAFLUSH, ORIG_RAW.0.get()) };
    }
}

/// SIGHUP/SIGINT/SIGTERM: hand the terminal back, then die of the signal so
/// that the exit status is the usual one.
extern "C" fn on_signal(sig: i32) {
    // SAFETY: every call below is async-signal-safe, so it is legal from a
    // handler. `restore_raw` only writes fd 1 and resets fd 0, which is what
    // this handler exists to do. `sig` was handed to us by the kernel, so it
    // is a real signal number, and the `SigSet` is local, initialised
    // and only ever passed as `&mut`/`&`, so the pointers are valid and
    // unaliased for the length of each call.
    unsafe {
        restore_raw();
        // Back to the default disposition, unblock the signal (it is blocked
        // while its own handler runs) and take it properly, so that the shell
        // sees an ordinary death by signal. `_exit` covers the case where
        // something still swallows it.
        signal(sig, SIG_DFL);
        let mut set = EMPTY_SIGSET;
        if sigemptyset(&mut set) == 0 && sigaddset(&mut set, sig) == 0 {
            sigprocmask(SIG_UNBLOCK, &set, std::ptr::null_mut());
        }
        raise(sig);
        _exit(128 + sig);
    }
}

/// Install `on_signal` for `sig`, unless the signal was inherited as ignored
/// (a daemonising parent's doing), in which case it stays ignored.
///
/// # Safety
///
/// Changes process-wide signal dispositions.
unsafe fn install(sig: i32) {
    // SAFETY: `on_signal` is an `extern "C"` function of the shape `signal(2)`
    // expects, and its address outlives the process; `SIG_IGN` is the constant
    // libc defines for the same parameter. The caller has accepted that the
    // process-wide disposition of `sig` changes.
    unsafe {
        let prev = signal(sig, on_signal as *const () as usize);
        if prev == SIG_IGN {
            signal(sig, SIG_IGN);
        }
    }
}

/// Whether both stdin and stdout are terminals.
pub fn is_tty() -> bool {
    // SAFETY: `isatty` only inspects the descriptor it is given and has no
    // preconditions beyond that; 0 and 1 are always valid to ask about.
    unsafe { isatty(0) == 1 && isatty(1) == 1 }
}

/// The terminal's size in columns and rows, falling back to 80x24.
pub fn size() -> (usize, usize) {
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: `TIOCGWINSZ` takes one `struct winsize *` out-parameter, and
    // `ws` is a live, correctly laid out (see the module header) `Winsize`
    // that nothing else aliases for the length of the call.
    let r = unsafe { ioctl(1, TIOCGWINSZ, &mut ws as *mut Winsize) };
    if r != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        (80, 24)
    } else {
        (ws.ws_col as usize, ws.ws_row as usize)
    }
}

/// Turn xterm mouse reporting (SGR encoding) on or off.
pub fn set_mouse(on: bool) {
    let mut out = io::stdout();
    let _ = out.write_all(if on {
        b"\x1b[?1000h\x1b[?1006h"
    } else {
        b"\x1b[?1006l\x1b[?1000l"
    });
    let _ = out.flush();
}

/// Enter raw mode and the alternate screen. Returns false if not a tty.
pub fn enter(mouse: bool) -> bool {
    if !is_tty() {
        return false;
    }
    let mut t = Termios::ZERO;
    // SAFETY: `t` and `raw` are live, correctly laid out (see the module
    // header) `Termios` values that nothing else aliases, so `tcgetattr` may
    // fill one in and `tcsetattr` may read the other. The `ORIG_RAW` write is
    // the single one allowed by `ORIG_RAW_CLAIM`, it happens before the
    // release of `ORIG_RAW_SET` that any reader acquires, and it is done
    // before `install` makes a reader (the signal handler) possible.
    unsafe {
        if tcgetattr(0, &mut t) != 0 {
            return false;
        }
        // Save the settings in both places *before* installing the handlers
        // that read the raw copy, and write that copy only once.
        if !ORIG_RAW_CLAIM.swap(true, Ordering::SeqCst) {
            ORIG_RAW.0.get().write(t);
            ORIG_RAW_SET.store(true, Ordering::Release);
        }
        let _ = ORIG.set(t);
        if !HANDLERS.swap(true, Ordering::SeqCst) {
            install(SIGHUP);
            install(SIGINT);
            install(SIGTERM);
        }
        let mut raw = t;
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !OPOST;
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 0;
        if tcsetattr(0, TCSAFLUSH, &raw) != 0 {
            return false;
        }
    }
    let mut out = io::stdout();
    // Alternate screen, hide cursor, clear, bracketed paste on.
    let _ = out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H\x1b[?2004h");
    let _ = out.flush();
    if mouse {
        set_mouse(true);
    }
    true
}

/// Leave raw mode and the alternate screen. Safe to call more than once, and
/// safe from the panic hook: it neither allocates nor waits on a lock of ours.
pub fn leave() {
    let mut out = io::stdout();
    let _ = out.write_all(RESET);
    let _ = out.flush();
    if let Some(t) = ORIG.get() {
        // SAFETY: `t` borrows a settled `OnceLock` value, so it points at an
        // initialised `Termios` for the whole call, and `tcsetattr` only reads
        // through it.
        unsafe {
            tcsetattr(0, TCSAFLUSH, t);
        }
    }
}

/// One keypress, as the escape-sequence parser understood it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    /// A bracketed paste, delivered whole so that its contents never look
    /// like commands.
    Paste(String),
    Ctrl(char),
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    F(u8),
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    CtrlUp,
    CtrlDown,
    CtrlLeft,
    CtrlRight,
    Mouse(Mouse),
}

/// What the mouse did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseKind {
    Press(u8),
    Release,
    Drag,
    WheelUp,
    WheelDown,
}

/// A mouse report: what happened, and where on the screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mouse {
    /// What the mouse did.
    pub kind: MouseKind,
    /// Column, zero-based.
    pub x: usize,
    /// Row, zero-based.
    pub y: usize,
}

/// Read whatever is waiting on stdin. Returns false once stdin is at end of
/// file; a read cut short by a signal is simply retried.
fn read_bytes(buf: &mut Vec<u8>) -> bool {
    let mut tmp = [0u8; 1024];
    loop {
        // SAFETY: `tmp` is a live, uniquely borrowed buffer and the count is
        // its true length, so `read` writes at most that many bytes into it.
        let n = unsafe { read(0, tmp.as_mut_ptr().cast(), tmp.len()) };
        if n > 0 {
            buf.extend_from_slice(&tmp[..n as usize]);
            return true;
        }
        if n == 0 {
            return false;
        }
        if errno() == EINTR {
            continue;
        }
        // EAGAIN and friends: nothing to add, but stdin is still there.
        return true;
    }
}

/// What a `poll` of stdin came back with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ready {
    Input,
    Timeout,
    Closed,
}

fn wait_input(timeout_ms: i32) -> Ready {
    let mut pfd = PollFd {
        fd: 0,
        events: POLLIN,
        revents: 0,
    };
    // SAFETY: `pfd` is one live, correctly laid out `struct pollfd` that
    // nothing else aliases, and the count passed says so.
    let r = unsafe { poll(&mut pfd, 1, timeout_ms) };
    if r <= 0 {
        // Nothing yet, or interrupted by a signal (a window resize, say): the
        // caller redraws and comes back, so this counts as a timeout.
        return Ready::Timeout;
    }
    if pfd.revents & POLLIN != 0 {
        Ready::Input
    } else if pfd.revents & (POLLHUP | POLLERR | POLLNVAL) != 0 {
        Ready::Closed
    } else {
        Ready::Timeout
    }
}

fn has_input(timeout_ms: i32) -> bool {
    wait_input(timeout_ms) == Ready::Input
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// Buffered stdin, handing out one [`Key`] at a time.
pub struct Input {
    buf: Vec<u8>,
    /// A paste stays data until its closing marker, including across timeouts.
    paste: Option<Vec<u8>>,
    /// Stdin has closed: there will never be another key.
    eof: bool,
}

impl Input {
    /// An empty buffer over stdin.
    pub fn new() -> Input {
        Input {
            buf: Vec::new(),
            paste: None,
            eof: false,
        }
    }

    /// Wait up to `timeout_ms` for a key. Returns None on timeout.
    pub fn poll_key(&mut self, timeout_ms: i32) -> Option<Key> {
        if self.paste.is_some() {
            if let Some(key) = self.take_paste() {
                return Some(key);
            }
        }
        if self.buf.is_empty() || self.paste.is_some() {
            if self.eof {
                // Stdin is gone. `poll` would return at once, for ever, so
                // wait out the caller's timeout by hand instead of spinning.
                if timeout_ms > 0 {
                    std::thread::sleep(Duration::from_millis(timeout_ms.min(1000) as u64));
                }
                return None;
            }
            match wait_input(timeout_ms) {
                Ready::Timeout => return None,
                Ready::Closed => {
                    self.eof = true;
                    return None;
                }
                Ready::Input => {
                    if !read_bytes(&mut self.buf) {
                        self.eof = true;
                    }
                }
            }
            if self.buf.is_empty() {
                return None;
            }
        }
        self.parse()
    }

    /// Consume available paste bytes without blocking the UI. A delay must
    /// never turn the remaining text into commands. Capture at most 256 KiB,
    /// discarding excess bytes until the closing marker (or EOF).
    fn take_paste(&mut self) -> Option<Key> {
        const END: &[u8] = b"\x1b[201~";
        const MAX: usize = 256 * 1024;
        let end = find(&self.buf, END);
        // Retain enough bytes for a closing marker split across reads.
        let take = end.unwrap_or_else(|| {
            if self.eof {
                self.buf.len()
            } else {
                self.buf.len().saturating_sub(END.len() - 1)
            }
        });
        let text = self.paste.as_mut()?;
        let capture = take.min(MAX.saturating_sub(text.len()));
        text.extend_from_slice(&self.buf[..capture]);
        self.buf
            .drain(..take + if end.is_some() { END.len() } else { 0 });
        if end.is_some() || self.eof {
            let text = self.paste.take().unwrap();
            Some(Key::Paste(String::from_utf8_lossy(&text).into_owned()))
        } else {
            None
        }
    }

    fn parse(&mut self) -> Option<Key> {
        if self.paste.is_some() {
            return self.take_paste();
        }
        let b0 = self.buf[0];
        if b0 == 0x1b {
            if self.buf.len() == 1 {
                // Lone escape or the start of a sequence still in flight.
                if has_input(15) {
                    read_bytes(&mut self.buf);
                }
                if self.buf.len() == 1 {
                    self.buf.remove(0);
                    return Some(Key::Esc);
                }
            }
            let b1 = self.buf[1];
            if b1 == b'[' || b1 == b'O' {
                // CSI / SS3 sequence: gather until a final byte in 0x40..=0x7e.
                let mut end = 2;
                while end < self.buf.len() && !(0x40..=0x7e).contains(&self.buf[end]) {
                    end += 1;
                }
                if end >= self.buf.len() {
                    if has_input(15) {
                        read_bytes(&mut self.buf);
                    }
                    while end < self.buf.len() && !(0x40..=0x7e).contains(&self.buf[end]) {
                        end += 1;
                    }
                    if end >= self.buf.len() {
                        self.buf.clear();
                        return Some(Key::Esc);
                    }
                }
                let seq: Vec<u8> = self.buf.drain(..=end).collect();
                let fin = seq[end];
                let params = &seq[2..end];
                // SGR mouse report: ESC [ < b ; x ; y M/m
                if b1 == b'[' && params.first() == Some(&b'<') && (fin == b'M' || fin == b'm') {
                    let text = std::str::from_utf8(&params[1..]).unwrap_or("");
                    let mut it = text.split(';').map(|v| v.parse::<usize>().unwrap_or(0));
                    let cb = it.next().unwrap_or(0);
                    let cx = it.next().unwrap_or(1);
                    let cy = it.next().unwrap_or(1);
                    let kind = if cb & 64 != 0 {
                        if cb & 1 != 0 {
                            MouseKind::WheelDown
                        } else {
                            MouseKind::WheelUp
                        }
                    } else if fin == b'm' {
                        MouseKind::Release
                    } else if cb & 32 != 0 {
                        MouseKind::Drag
                    } else {
                        MouseKind::Press((cb & 3) as u8)
                    };
                    return Some(Key::Mouse(Mouse {
                        kind,
                        x: cx.saturating_sub(1),
                        y: cy.saturating_sub(1),
                    }));
                }
                let mut fields = params.split(|&b| b == b';');
                let first = fields.next().unwrap_or(&[]);
                // Bracketed paste: ESC [ 200 ~ text ESC [ 201 ~
                if b1 == b'[' && fin == b'~' && first == b"200" {
                    self.paste = Some(Vec::new());
                    return self.take_paste();
                }
                let modifier = fields
                    .next()
                    .and_then(|m| std::str::from_utf8(m).ok())
                    .and_then(|m| m.parse::<u8>().ok())
                    .unwrap_or(1);
                let shift = matches!(modifier, 2 | 4 | 6 | 8);
                let ctrl = matches!(modifier, 5..=8);
                let arrow = |plain: Key, s: Key, c: Key| {
                    if ctrl {
                        c
                    } else if shift {
                        s
                    } else {
                        plain
                    }
                };
                let key = match fin {
                    b'A' => arrow(Key::Up, Key::ShiftUp, Key::CtrlUp),
                    b'B' => arrow(Key::Down, Key::ShiftDown, Key::CtrlDown),
                    b'C' => arrow(Key::Right, Key::ShiftRight, Key::CtrlRight),
                    b'D' => arrow(Key::Left, Key::ShiftLeft, Key::CtrlLeft),
                    b'H' => Key::Home,
                    b'F' => Key::End,
                    b'Z' => Key::BackTab,
                    b'P' => Key::F(1),
                    b'Q' => Key::F(2),
                    b'R' => Key::F(3),
                    b'S' => Key::F(4),
                    b'~' => match first {
                        b"1" | b"7" => Key::Home,
                        b"4" | b"8" => Key::End,
                        b"3" => Key::Delete,
                        b"5" => Key::PageUp,
                        b"6" => Key::PageDown,
                        b"11" => Key::F(1),
                        b"12" => Key::F(2),
                        b"13" => Key::F(3),
                        b"14" => Key::F(4),
                        b"15" => Key::F(5),
                        _ => Key::Esc,
                    },
                    _ => Key::Esc,
                };
                return Some(key);
            }
            // Alt+key: treat as Esc followed by the key.
            self.buf.remove(0);
            return Some(Key::Esc);
        }
        // UTF-8 decode a single scalar.
        let len = if b0 < 0x80 {
            1
        } else if b0 >> 5 == 0b110 {
            2
        } else if b0 >> 4 == 0b1110 {
            3
        } else {
            4
        };
        if self.buf.len() < len {
            self.buf.clear();
            return None;
        }
        let bytes: Vec<u8> = self.buf.drain(..len).collect();
        let s = String::from_utf8_lossy(&bytes);
        let c = s.chars().next()?;
        Some(match c {
            '\r' | '\n' => Key::Enter,
            '\t' => Key::Tab,
            '\x7f' | '\x08' => Key::Backspace,
            // The ASCII control block, back to the key that made it: setting
            // bit 6 turns 0x01 into 'A' and so on, and lowercasing gives the
            // `Ctrl('d')` spelling the rest of the tree matches on. Done by
            // hand it is an easy subtraction to get wrong — a NUL byte
            // (Ctrl+Space on most terminals) underflows `b'a' + c - 1`.
            c if (c as u32) < 0x20 => Key::Ctrl(((c as u8) | 0x40).to_ascii_lowercase() as char),
            c => Key::Char(c),
        })
    }
}

/// A 24-bit colour: red, green, blue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(
    /// Red.
    pub u8,
    /// Green.
    pub u8,
    /// Blue.
    pub u8,
);

impl Rgb {
    /// Linear blend towards `o`; `t` is clamped to `[0, 1]`.
    pub fn mix(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            (self.0 as f32 + (o.0 as f32 - self.0 as f32) * t) as u8,
            (self.1 as f32 + (o.1 as f32 - self.1 as f32) * t) as u8,
            (self.2 as f32 + (o.2 as f32 - self.2 as f32) * t) as u8,
        )
    }
    /// Every channel multiplied by `k` and clamped back into range.
    pub fn scale(self, k: f32) -> Rgb {
        Rgb(
            (self.0 as f32 * k).clamp(0.0, 255.0) as u8,
            (self.1 as f32 * k).clamp(0.0, 255.0) as u8,
            (self.2 as f32 * k).clamp(0.0, 255.0) as u8,
        )
    }
    /// From hue (0-360), saturation and value (both 0-1).
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Rgb {
        let h = h.rem_euclid(360.0);
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Rgb(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
    /// Hue (0-360), saturation and value, all but hue in 0..1.
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.0 as f32 / 255.0;
        let g = self.1 as f32 / 255.0;
        let b = self.2 as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let d = max - min;
        let h = if d < 1e-6 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max < 1e-6 { 0.0 } else { d / max };
        (h.rem_euclid(360.0), s, max)
    }

    /// Perceived brightness in `[0, 1]`, for deciding on light or dark text.
    pub fn luma(self) -> f32 {
        (0.299 * self.0 as f32 + 0.587 * self.1 as f32 + 0.114 * self.2 as f32) / 255.0
    }
}

/// Attribute bit: bold.
pub const BOLD: u8 = 1;
/// Attribute bit: dim.
pub const DIM: u8 = 2;
/// Attribute bit: italic.
pub const ITALIC: u8 = 4;
/// Attribute bit: underline.
pub const UNDERLINE: u8 = 8;
/// Attribute bit: reverse video.
pub const REVERSE: u8 = 16;

/// A rectangle of character cells: the top-left corner and a size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    /// Column of the left edge.
    pub x: usize,
    /// Row of the top edge.
    pub y: usize,
    /// Width in columns.
    pub w: usize,
    /// Height in rows.
    pub h: usize,
}

impl Rect {
    /// A rectangle `w` by `h` with its top-left corner at (`x`, `y`).
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Rect {
        Rect { x, y, w, h }
    }
}

/// How something is painted: a foreground colour, a background colour and a
/// bitmask of the `BOLD`..`REVERSE` attributes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Style {
    /// Text colour.
    pub fg: Rgb,
    /// Background colour.
    pub bg: Rgb,
    /// `BOLD | DIM | ITALIC | UNDERLINE | REVERSE`, or 0 for plain.
    pub attr: u8,
}

impl Style {
    /// Plain text in these colours: no bold, dim, italic or underline.
    pub fn new(fg: Rgb, bg: Rgb) -> Style {
        Style { fg, bg, attr: 0 }
    }

    /// These colours with the attribute bits in `attr` set.
    pub fn attr(fg: Rgb, bg: Rgb, attr: u8) -> Style {
        Style { fg, bg, attr }
    }
}

/// One character cell of the screen buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character drawn in the cell.
    pub ch: char,
    /// Its colour.
    pub fg: Rgb,
    /// The colour behind it.
    pub bg: Rgb,
    /// Its attribute bits.
    pub attr: u8,
}

impl Cell {
    /// An empty cell over `bg`.
    pub fn blank(bg: Rgb) -> Cell {
        Cell {
            ch: ' ',
            fg: Rgb(200, 200, 200),
            bg,
            attr: 0,
        }
    }
}

/// A cell buffer that redraws only what changed since the last [`Screen::flush`].
pub struct Screen {
    /// Width in columns.
    pub w: usize,
    /// Height in rows.
    pub h: usize,
    cells: Vec<Cell>,
    prev: Vec<Cell>,
    dirty_all: bool,
    out: String,
}

impl Screen {
    /// A blank screen `w` by `h`.
    pub fn new(w: usize, h: usize) -> Screen {
        let bg = Rgb(0, 0, 0);
        Screen {
            w,
            h,
            cells: vec![Cell::blank(bg); w * h],
            prev: vec![Cell::blank(bg); w * h],
            dirty_all: true,
            out: String::with_capacity(w * h * 8),
        }
    }

    /// Change the size, blanking everything, if the size really changed.
    pub fn resize(&mut self, w: usize, h: usize) {
        if w != self.w || h != self.h {
            self.w = w;
            self.h = h;
            let bg = Rgb(0, 0, 0);
            self.cells = vec![Cell::blank(bg); w * h];
            self.prev = vec![Cell::blank(bg); w * h];
            self.dirty_all = true;
        }
    }

    /// Blank every cell over `bg`.
    pub fn clear(&mut self, bg: Rgb) {
        for c in self.cells.iter_mut() {
            *c = Cell::blank(bg);
        }
    }

    /// Write one character. Out-of-range positions are ignored.
    #[inline]
    pub fn put(&mut self, x: usize, y: usize, ch: char, fg: Rgb, bg: Rgb) {
        if x < self.w && y < self.h {
            self.cells[y * self.w + x] = Cell {
                ch,
                fg,
                bg,
                attr: 0,
            };
        }
    }

    /// Write one character with attributes. Out-of-range positions are ignored.
    #[inline]
    pub fn put_attr(&mut self, x: usize, y: usize, ch: char, fg: Rgb, bg: Rgb, attr: u8) {
        if x < self.w && y < self.h {
            self.cells[y * self.w + x] = Cell { ch, fg, bg, attr };
        }
    }

    /// The whole buffer, for a pass that re-colours a finished frame.
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    /// The cell at (`x`, `y`), or a blank one if that is off the screen.
    pub fn cell(&self, x: usize, y: usize) -> Cell {
        if x < self.w && y < self.h {
            self.cells[y * self.w + x]
        } else {
            Cell::blank(Rgb(0, 0, 0))
        }
    }

    /// Write text; returns the x position after the last character written.
    pub fn text(&mut self, x: usize, y: usize, s: &str, fg: Rgb, bg: Rgb) -> usize {
        self.text_attr(x, y, s, fg, bg, 0)
    }

    /// Write text with attributes; returns the x position after it.
    pub fn text_attr(&mut self, x: usize, y: usize, s: &str, fg: Rgb, bg: Rgb, attr: u8) -> usize {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.w {
                break;
            }
            self.put_attr(cx, y, ch, fg, bg, attr);
            cx += 1;
        }
        cx
    }

    /// Write text clipped to `max` columns. Returns the x position after the
    /// last character written.
    pub fn text_clip(&mut self, x: usize, y: usize, s: &str, max: usize, st: Style) -> usize {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.w || cx >= x + max {
                break;
            }
            self.put_attr(cx, y, ch, st.fg, st.bg, st.attr);
            cx += 1;
        }
        cx
    }

    /// Paint every cell of `r` with `ch`.
    pub fn fill(&mut self, r: Rect, ch: char, st: Style) {
        for yy in r.y..(r.y + r.h).min(self.h) {
            for xx in r.x..(r.x + r.w).min(self.w) {
                self.cells[yy * self.w + xx] = Cell {
                    ch,
                    fg: st.fg,
                    bg: st.bg,
                    attr: st.attr,
                };
            }
        }
    }

    /// Draw `w` columns of horizontal rule starting at (`x`, `y`).
    pub fn hline(&mut self, x: usize, y: usize, w: usize, fg: Rgb, bg: Rgb) {
        for xx in x..(x + w).min(self.w) {
            self.put(xx, y, '─', fg, bg);
        }
    }

    /// Draw `h` rows of vertical rule starting at (`x`, `y`).
    pub fn vline(&mut self, x: usize, y: usize, h: usize, fg: Rgb, bg: Rgb) {
        for yy in y..(y + h).min(self.h) {
            self.put(x, yy, '│', fg, bg);
        }
    }

    /// Draw a box around `r`, with `title` inset into its top edge if it fits.
    pub fn frame(&mut self, r: Rect, title: &str, st: Style) {
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        let (fg, bg) = (st.fg, st.bg);
        if w < 2 || h < 2 {
            return;
        }
        self.hline(x, y, w, fg, bg);
        self.hline(x, y + h - 1, w, fg, bg);
        self.vline(x, y, h, fg, bg);
        self.vline(x + w - 1, y, h, fg, bg);
        self.put(x, y, '┌', fg, bg);
        self.put(x + w - 1, y, '┐', fg, bg);
        self.put(x, y + h - 1, '└', fg, bg);
        self.put(x + w - 1, y + h - 1, '┘', fg, bg);
        if !title.is_empty() && w > title.chars().count() + 4 {
            let t = format!(" {} ", title);
            self.text_attr(x + 2, y, &t, fg, bg, st.attr | BOLD);
        }
    }

    fn sgr(out: &mut String, c: &Cell, last: &mut Option<(Rgb, Rgb, u8)>) {
        let cur = (c.fg, c.bg, c.attr);
        if *last == Some(cur) {
            return;
        }
        let attr_changed = last.map(|l| l.2 != c.attr).unwrap_or(true);
        if attr_changed {
            out.push_str("\x1b[0");
            if c.attr & BOLD != 0 {
                out.push_str(";1");
            }
            if c.attr & DIM != 0 {
                out.push_str(";2");
            }
            if c.attr & ITALIC != 0 {
                out.push_str(";3");
            }
            if c.attr & UNDERLINE != 0 {
                out.push_str(";4");
            }
            if c.attr & REVERSE != 0 {
                out.push_str(";7");
            }
            out.push('m');
        }
        if attr_changed || last.map(|l| l.0 != c.fg).unwrap_or(true) {
            out.push_str(&format!("\x1b[38;2;{};{};{}m", c.fg.0, c.fg.1, c.fg.2));
        }
        if attr_changed || last.map(|l| l.1 != c.bg).unwrap_or(true) {
            out.push_str(&format!("\x1b[48;2;{};{};{}m", c.bg.0, c.bg.1, c.bg.2));
        }
        *last = Some(cur);
    }

    /// Send everything that changed since the last flush to the terminal.
    pub fn flush(&mut self) {
        self.out.clear();
        let mut last: Option<(Rgb, Rgb, u8)> = None;
        if self.dirty_all {
            self.out.push_str("\x1b[0m\x1b[2J");
        }
        for y in 0..self.h {
            let mut cursor_at: Option<usize> = None;
            for x in 0..self.w {
                let i = y * self.w + x;
                let c = self.cells[i];
                if !self.dirty_all && c == self.prev[i] {
                    continue;
                }
                if cursor_at != Some(x) {
                    self.out.push_str(&format!("\x1b[{};{}H", y + 1, x + 1));
                }
                Screen::sgr(&mut self.out, &c, &mut last);
                self.out.push(c.ch);
                cursor_at = Some(x + 1);
            }
        }
        self.out.push_str("\x1b[0m");
        let stdout = io::stdout();
        let mut lock = stdout.lock();
        let _ = lock.write_all(self.out.as_bytes());
        let _ = lock.flush();
        self.prev.copy_from_slice(&self.cells);
        self.dirty_all = false;
    }
}

/// Word-wrap `s` into lines of at most `width` chars.
pub fn wrap(s: &str, width: usize) -> Vec<String> {
    let width = width.max(4);
    let mut lines = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        let mut len = 0;
        for word in para.split_whitespace() {
            let wl = word.chars().count();
            if len > 0 && len + 1 + wl > width {
                lines.push(std::mem::take(&mut line));
                len = 0;
            }
            if len > 0 {
                line.push(' ');
                len += 1;
            }
            line.push_str(word);
            len += wl;
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drain every key an already-buffered byte string decodes to. Only
    /// sequences that are complete in the buffer are used, so the parser
    /// never has to go back to the real stdin.
    fn keys(bytes: &[u8]) -> Vec<Key> {
        let mut input = Input {
            buf: bytes.to_vec(),
            paste: None,
            eof: true,
        };
        let mut out = Vec::new();
        while !input.buf.is_empty() {
            match input.parse() {
                Some(k) => out.push(k),
                None => break,
            }
        }
        out
    }

    #[test]
    fn plain_characters_and_the_named_keys() {
        assert_eq!(
            keys(b"ab\r\t\x7f"),
            vec![
                Key::Char('a'),
                Key::Char('b'),
                Key::Enter,
                Key::Tab,
                Key::Backspace
            ]
        );
    }

    #[test]
    fn control_characters_never_overflow() {
        // Ctrl+D and Ctrl+U are load-bearing: half-page scrolling.
        assert_eq!(keys(b"\x04"), vec![Key::Ctrl('d')]);
        assert_eq!(keys(b"\x15"), vec![Key::Ctrl('u')]);
        assert_eq!(keys(b"\x02\x06"), vec![Key::Ctrl('b'), Key::Ctrl('f')]);
        // Every byte in the control block must decode to something rather
        // than panicking. A NUL (Ctrl+Space on most terminals) used to
        // underflow the arithmetic that produced the letter.
        for b in 0u8..0x20 {
            if b == 0x1b {
                continue; // escape: a sequence, tested below
            }
            assert!(!keys(&[b]).is_empty(), "byte {:#04x} decoded to nothing", b);
        }
    }

    #[test]
    fn utf8_survives_the_decoder() {
        assert_eq!(keys("é".as_bytes()), vec![Key::Char('é')]);
        assert_eq!(keys("—x".as_bytes()), vec![Key::Char('—'), Key::Char('x')]);
    }

    #[test]
    fn arrows_carry_their_modifiers() {
        assert_eq!(
            keys(b"\x1b[A\x1b[B\x1b[C\x1b[D"),
            vec![Key::Up, Key::Down, Key::Right, Key::Left]
        );
        assert_eq!(keys(b"\x1b[1;2A"), vec![Key::ShiftUp]);
        assert_eq!(keys(b"\x1b[1;5D"), vec![Key::CtrlLeft]);
        assert_eq!(keys(b"\x1b[Z"), vec![Key::BackTab]);
        assert_eq!(keys(b"\x1b[5~\x1b[6~"), vec![Key::PageUp, Key::PageDown]);
        assert_eq!(keys(b"\x1b[H\x1b[F"), vec![Key::Home, Key::End]);
    }

    #[test]
    fn sgr_mouse_reports_decode() {
        assert_eq!(
            keys(b"\x1b[<0;10;5M"),
            vec![Key::Mouse(Mouse {
                kind: MouseKind::Press(0),
                x: 9,
                y: 4
            })]
        );
        assert_eq!(
            keys(b"\x1b[<0;10;5m"),
            vec![Key::Mouse(Mouse {
                kind: MouseKind::Release,
                x: 9,
                y: 4
            })]
        );
        assert_eq!(
            keys(b"\x1b[<64;1;1M"),
            vec![Key::Mouse(Mouse {
                kind: MouseKind::WheelUp,
                x: 0,
                y: 0
            })]
        );
        assert_eq!(
            keys(b"\x1b[<65;1;1M"),
            vec![Key::Mouse(Mouse {
                kind: MouseKind::WheelDown,
                x: 0,
                y: 0
            })]
        );
    }

    #[test]
    fn a_paste_arrives_whole() {
        // Text that would otherwise read as ":q<Enter>" must come back as one
        // string, so that pasting into a prompt cannot run commands.
        let got = keys(b"\x1b[200~:q\r\x1b[201~x");
        assert_eq!(
            got,
            vec![Key::Paste(":q\r".to_string()), Key::Char('x')],
            "a bracketed paste must be delivered whole"
        );
    }

    #[test]
    fn a_partial_paste_stays_text_until_the_split_closing_marker() {
        let mut input = Input::new();
        input.buf.extend_from_slice(b"\x1b[200~");
        assert_eq!(input.parse(), None);
        input.buf.extend_from_slice(b":q!\r");
        assert_eq!(input.parse(), None);
        input.buf.extend_from_slice(b"\x1b[20");
        assert_eq!(input.parse(), None);
        input.buf.extend_from_slice(b"1~x");
        assert_eq!(input.parse(), Some(Key::Paste(":q!\r".into())));
        assert_eq!(input.parse(), Some(Key::Char('x')));
    }

    #[test]
    fn an_oversized_paste_discards_excess_without_executing_it() {
        let mut input = Input::new();
        input.buf.extend_from_slice(b"\x1b[200~");
        input.buf.extend(vec![b'a'; 256 * 1024 + 100]);
        assert_eq!(input.parse(), None);
        input.buf.extend_from_slice(b":q!\r\x1b[201~z");
        assert_eq!(input.parse(), Some(Key::Paste("a".repeat(256 * 1024))));
        assert_eq!(input.parse(), Some(Key::Char('z')));
    }

    #[test]
    fn rubbish_on_the_wire_does_not_panic() {
        // Every byte, alone and behind an escape: the parser may return
        // anything it likes, but it may not panic or loop for ever.
        for b in 0u8..=255 {
            let _ = keys(&[b]);
            let _ = keys(&[0x1b, b'[', b]);
            let _ = keys(&[0x1b, b'O', b]);
            let _ = keys(&[b, b'x']);
        }
        let _ = keys(b"\x1b[<;;;;;;M");
        let _ = keys(b"\x1b[999999999999999999999;1;1M");
        let _ = keys(b"\x1b[200~unterminated paste");
    }

    #[test]
    fn wrapping_keeps_every_word() {
        let lines = wrap("the quick brown fox jumps over the lazy dog", 12);
        assert!(lines.iter().all(|l| l.chars().count() <= 12), "{:?}", lines);
        assert_eq!(
            lines.join(" "),
            "the quick brown fox jumps over the lazy dog"
        );
        // Blank lines in the input are kept, so paragraphs stay apart.
        assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
        // A word longer than the width is not lost.
        assert_eq!(
            wrap("antidisestablishmentarianism", 5).join(""),
            "antidisestablishmentarianism"
        );
    }

    #[test]
    fn a_screen_only_writes_where_it_can() {
        let mut s = Screen::new(10, 3);
        // Out-of-range writes are dropped rather than panicking.
        s.put(100, 100, 'x', Rgb(0, 0, 0), Rgb(0, 0, 0));
        s.put(10, 0, 'x', Rgb(0, 0, 0), Rgb(0, 0, 0));
        assert_eq!(s.cell(100, 100).ch, ' ');
        // Text is clipped at the right edge, not wrapped onto the next row.
        s.text(6, 1, "abcdefgh", Rgb(255, 255, 255), Rgb(0, 0, 0));
        assert_eq!(s.cell(9, 1).ch, 'd');
        assert_eq!(s.cell(0, 2).ch, ' ');
        // A frame smaller than its own border is refused rather than drawn.
        s.frame(
            Rect::new(0, 0, 1, 1),
            "t",
            Style::new(Rgb(1, 1, 1), Rgb(0, 0, 0)),
        );
        assert_eq!(s.cell(0, 0).ch, ' ');
    }

    #[test]
    fn colours_round_trip_through_hsv() {
        for c in [
            Rgb(255, 0, 0),
            Rgb(0, 128, 64),
            Rgb(12, 34, 56),
            Rgb(0, 0, 0),
        ] {
            let (h, s, v) = c.to_hsv();
            let back = Rgb::from_hsv(h, s, v);
            for (a, b) in [(c.0, back.0), (c.1, back.1), (c.2, back.2)] {
                assert!(a.abs_diff(b) <= 2, "{:?} became {:?}", c, back);
            }
        }
        assert_eq!(Rgb(10, 20, 30).mix(Rgb(10, 20, 30), 5.0), Rgb(10, 20, 30));
        assert_eq!(Rgb(200, 200, 200).scale(10.0), Rgb(255, 255, 255));
    }
}
