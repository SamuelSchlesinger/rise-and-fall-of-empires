//! The libc ABI used by the terminal backend. Keep platform differences here.
//!
//! Linux glibc/musl and macOS are supported on 64-bit `x86_64` and aarch64.
//! In particular, Darwin's termios flags/speeds are unsigned longs, its
//! control-character array has 20 entries and no `c_line`, and `sigset_t` is u32.
//! These definitions follow the native termios.h, poll.h, signal.h and ioctl.h
//! headers; Apple's sources are at <https://github.com/apple-oss-distributions/xnu/tree/main/bsd/sys>.
//! Other targets must supply their own ABI before this module can be used.

#[cfg(not(all(
    any(
        all(target_os = "linux", any(target_env = "gnu", target_env = "musl")),
        target_os = "macos"
    ),
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_pointer_width = "64"
)))]
compile_error!(
    "the terminal backend supports 64-bit x86_64/aarch64 Linux (glibc/musl) and macOS; \
     port src/term/sys.rs before building for this target"
);

use std::ffi::{c_ulong, c_void};

#[cfg(target_os = "linux")]
mod platform {
    pub type TcFlag = u32;
    pub type Speed = u32;
    pub type Nfds = std::ffi::c_ulong;
    pub type SigSet = [u64; 16];
    pub const EMPTY_SIGSET: SigSet = [0; 16];
    pub const NCCS: usize = 32;
    pub const ISIG: TcFlag = 0o000001;
    pub const ICANON: TcFlag = 0o000002;
    pub const IEXTEN: TcFlag = 0o100000;
    pub const IXON: TcFlag = 0o002000;
    pub const VTIME: usize = 5;
    pub const VMIN: usize = 6;
    pub const TIOCGWINSZ: std::ffi::c_ulong = 0x5413;
    pub const SIG_UNBLOCK: i32 = 1;
}

#[cfg(target_os = "macos")]
mod platform {
    pub type TcFlag = std::ffi::c_ulong;
    pub type Speed = std::ffi::c_ulong;
    pub type Nfds = std::ffi::c_uint;
    pub type SigSet = u32;
    pub const EMPTY_SIGSET: SigSet = 0;
    pub const NCCS: usize = 20;
    pub const ISIG: TcFlag = 0x00000080;
    pub const ICANON: TcFlag = 0x00000100;
    pub const IEXTEN: TcFlag = 0x00000400;
    pub const IXON: TcFlag = 0x00000200;
    pub const VTIME: usize = 17;
    pub const VMIN: usize = 16;
    pub const TIOCGWINSZ: std::ffi::c_ulong = 0x40087468;
    pub const SIG_UNBLOCK: i32 = 2;
}

pub(super) use platform::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Termios {
    pub c_iflag: TcFlag,
    pub c_oflag: TcFlag,
    pub c_cflag: TcFlag,
    pub c_lflag: TcFlag,
    #[cfg(target_os = "linux")]
    pub c_line: u8,
    pub c_cc: [u8; NCCS],
    pub c_ispeed: Speed,
    pub c_ospeed: Speed,
}

impl Termios {
    pub const ZERO: Self = Self {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        #[cfg(target_os = "linux")]
        c_line: 0,
        c_cc: [0; NCCS],
        c_ispeed: 0,
        c_ospeed: 0,
    };
}

#[repr(C)]
pub(super) struct Winsize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[repr(C)]
pub(super) struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

extern "C" {
    pub(super) fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
    pub(super) fn tcsetattr(fd: i32, opt: i32, t: *const Termios) -> i32;
    pub(super) fn ioctl(fd: i32, req: c_ulong, ...) -> i32;
    pub(super) fn poll(fds: *mut PollFd, nfds: Nfds, timeout: i32) -> i32;
    pub(super) fn read(fd: i32, buf: *mut c_void, count: usize) -> isize;
    pub(super) fn write(fd: i32, buf: *const c_void, count: usize) -> isize;
    pub(super) fn isatty(fd: i32) -> i32;
    /// The handler is an address, including the `SIG_DFL` and `SIG_IGN` sentinels.
    pub(super) fn signal(sig: i32, handler: usize) -> usize;
    pub(super) fn raise(sig: i32) -> i32;
    pub(super) fn sigemptyset(set: *mut SigSet) -> i32;
    pub(super) fn sigaddset(set: *mut SigSet, sig: i32) -> i32;
    pub(super) fn sigprocmask(how: i32, set: *const SigSet, old: *mut SigSet) -> i32;
    pub(super) fn _exit(code: i32) -> !;
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    #[cfg_attr(target_os = "macos", link_name = "__error")]
    pub(super) fn errno_location() -> *mut i32;
}

// These constants have the same values on all supported platforms.
pub(super) const ECHO: TcFlag = 0o000010;
pub(super) const ICRNL: TcFlag = 0o000400;
pub(super) const BRKINT: TcFlag = 0o000002;
pub(super) const INPCK: TcFlag = 0o000020;
pub(super) const ISTRIP: TcFlag = 0o000040;
pub(super) const OPOST: TcFlag = 0o000001;
pub(super) const TCSAFLUSH: i32 = 2;
pub(super) const POLLIN: i16 = 0x001;
pub(super) const POLLERR: i16 = 0x008;
pub(super) const POLLHUP: i16 = 0x010;
pub(super) const POLLNVAL: i16 = 0x020;
pub(super) const EINTR: i32 = 4;
pub(super) const SIG_DFL: usize = 0;
pub(super) const SIG_IGN: usize = 1;
pub(super) const SIGHUP: i32 = 1;
pub(super) const SIGINT: i32 = 2;
pub(super) const SIGTERM: i32 = 15;
