//! Minimal raw-mode terminal layer for Linux using libc FFI (no crates):
//! termios raw mode, window size, polled key input, and a diff-rendered
//! cell buffer emitting 24-bit ANSI colour.

use std::io::{self, Write};

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

extern "C" {
    fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, opt: i32, t: *const Termios) -> i32;
    fn ioctl(fd: i32, req: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn isatty(fd: i32) -> i32;
}

const ISIG: u32 = 0o000001;
const ICANON: u32 = 0o000002;
const ECHO: u32 = 0o000010;
const IEXTEN: u32 = 0o100000;
const IXON: u32 = 0o002000;
const ICRNL: u32 = 0o000400;
const BRKINT: u32 = 0o000002;
const INPCK: u32 = 0o000020;
const ISTRIP: u32 = 0o000040;
const OPOST: u32 = 0o000001;
const VTIME: usize = 5;
const VMIN: usize = 6;
const TCSAFLUSH: i32 = 2;
const TIOCGWINSZ: u64 = 0x5413;
const POLLIN: i16 = 0x001;

static mut ORIG: Option<Termios> = None;

pub fn is_tty() -> bool {
    unsafe { isatty(0) == 1 && isatty(1) == 1 }
}

pub fn size() -> (usize, usize) {
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
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
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    unsafe {
        if tcgetattr(0, &mut t) != 0 {
            return false;
        }
        ORIG = Some(t);
        let mut raw = t;
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !OPOST;
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 0;
        tcsetattr(0, TCSAFLUSH, &raw);
    }
    let mut out = io::stdout();
    // Alternate screen, hide cursor, clear.
    let _ = out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
    let _ = out.flush();
    if mouse {
        set_mouse(true);
    }
    true
}

pub fn leave() {
    let mut out = io::stdout();
    let _ = out.write_all(b"\x1b[?1006l\x1b[?1000l\x1b[0m\x1b[?25h\x1b[2J\x1b[H\x1b[?1049l");
    let _ = out.flush();
    unsafe {
        let orig = std::ptr::addr_of!(ORIG).read();
        if let Some(t) = orig {
            tcsetattr(0, TCSAFLUSH, &t);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseKind {
    Press(u8),
    Release,
    Drag,
    WheelUp,
    WheelDown,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mouse {
    pub kind: MouseKind,
    pub x: usize,
    pub y: usize,
}

fn read_bytes(buf: &mut Vec<u8>) {
    let mut tmp = [0u8; 64];
    let n = unsafe { read(0, tmp.as_mut_ptr(), tmp.len()) };
    if n > 0 {
        buf.extend_from_slice(&tmp[..n as usize]);
    }
}

fn wait_input(timeout_ms: i32) -> bool {
    let mut pfd = PollFd {
        fd: 0,
        events: POLLIN,
        revents: 0,
    };
    let r = unsafe { poll(&mut pfd, 1, timeout_ms) };
    r > 0 && (pfd.revents & POLLIN) != 0
}

pub struct Input {
    buf: Vec<u8>,
}

impl Input {
    pub fn new() -> Input {
        Input { buf: Vec::new() }
    }

    /// Wait up to `timeout_ms` for a key. Returns None on timeout.
    pub fn poll_key(&mut self, timeout_ms: i32) -> Option<Key> {
        if self.buf.is_empty() {
            if !wait_input(timeout_ms) {
                return None;
            }
            read_bytes(&mut self.buf);
            if self.buf.is_empty() {
                return None;
            }
        }
        self.parse()
    }

    fn parse(&mut self) -> Option<Key> {
        let b0 = self.buf[0];
        if b0 == 0x1b {
            if self.buf.len() == 1 {
                // Lone escape or the start of a sequence still in flight.
                if wait_input(15) {
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
                    if wait_input(15) {
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
                let modifier = fields
                    .next()
                    .and_then(|m| std::str::from_utf8(m).ok())
                    .and_then(|m| m.parse::<u8>().ok())
                    .unwrap_or(1);
                let shift = matches!(modifier, 2 | 4 | 6 | 8);
                let ctrl = matches!(modifier, 5 | 6 | 7 | 8);
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
            c if (c as u32) < 0x20 => Key::Ctrl((b'a' + (c as u8) - 1) as char),
            c => Key::Char(c),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn mix(self, o: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb(
            (self.0 as f32 + (o.0 as f32 - self.0 as f32) * t) as u8,
            (self.1 as f32 + (o.1 as f32 - self.1 as f32) * t) as u8,
            (self.2 as f32 + (o.2 as f32 - self.2 as f32) * t) as u8,
        )
    }
    pub fn scale(self, k: f32) -> Rgb {
        Rgb(
            (self.0 as f32 * k).clamp(0.0, 255.0) as u8,
            (self.1 as f32 * k).clamp(0.0, 255.0) as u8,
            (self.2 as f32 * k).clamp(0.0, 255.0) as u8,
        )
    }
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

    pub fn luma(self) -> f32 {
        (0.299 * self.0 as f32 + 0.587 * self.1 as f32 + 0.114 * self.2 as f32) / 255.0
    }
}

pub const BOLD: u8 = 1;
pub const DIM: u8 = 2;
pub const ITALIC: u8 = 4;
pub const UNDERLINE: u8 = 8;
pub const REVERSE: u8 = 16;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub attr: u8,
}

impl Cell {
    pub fn blank(bg: Rgb) -> Cell {
        Cell {
            ch: ' ',
            fg: Rgb(200, 200, 200),
            bg,
            attr: 0,
        }
    }
}

pub struct Screen {
    pub w: usize,
    pub h: usize,
    cells: Vec<Cell>,
    prev: Vec<Cell>,
    dirty_all: bool,
    out: String,
}

impl Screen {
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

    pub fn clear(&mut self, bg: Rgb) {
        for c in self.cells.iter_mut() {
            *c = Cell::blank(bg);
        }
    }

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

    #[inline]
    pub fn put_attr(&mut self, x: usize, y: usize, ch: char, fg: Rgb, bg: Rgb, attr: u8) {
        if x < self.w && y < self.h {
            self.cells[y * self.w + x] = Cell { ch, fg, bg, attr };
        }
    }

    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    pub fn cell(&self, x: usize, y: usize) -> Cell {
        self.cells[y * self.w + x]
    }

    /// Write text; returns the x position after the last character written.
    pub fn text(&mut self, x: usize, y: usize, s: &str, fg: Rgb, bg: Rgb) -> usize {
        self.text_attr(x, y, s, fg, bg, 0)
    }

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

    /// Write text clipped to `max` columns.
    pub fn text_clip(
        &mut self,
        x: usize,
        y: usize,
        s: &str,
        max: usize,
        fg: Rgb,
        bg: Rgb,
        attr: u8,
    ) -> usize {
        let mut cx = x;
        for ch in s.chars() {
            if cx >= self.w || cx >= x + max {
                break;
            }
            self.put_attr(cx, y, ch, fg, bg, attr);
            cx += 1;
        }
        cx
    }

    pub fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, ch: char, fg: Rgb, bg: Rgb) {
        for yy in y..(y + h).min(self.h) {
            for xx in x..(x + w).min(self.w) {
                self.cells[yy * self.w + xx] = Cell {
                    ch,
                    fg,
                    bg,
                    attr: 0,
                };
            }
        }
    }

    pub fn hline(&mut self, x: usize, y: usize, w: usize, fg: Rgb, bg: Rgb) {
        for xx in x..(x + w).min(self.w) {
            self.put(xx, y, '─', fg, bg);
        }
    }

    pub fn vline(&mut self, x: usize, y: usize, h: usize, fg: Rgb, bg: Rgb) {
        for yy in y..(y + h).min(self.h) {
            self.put(x, yy, '│', fg, bg);
        }
    }

    pub fn frame(&mut self, x: usize, y: usize, w: usize, h: usize, title: &str, fg: Rgb, bg: Rgb) {
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
            self.text_attr(x + 2, y, &t, fg, bg, BOLD);
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
