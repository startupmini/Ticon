//! Lebar terminal, tanpa dependensi.
//!
//! Dihitung di sini dan bukan lewat crate, karena crate sejenis menarik
//! `windows-sys` — dan di toolchain `windows-gnu` itu memaksa `dlltool` untuk
//! membuat import library. Satu angka saja tidak sepadan dengan tambahan
//! rantai alat seperti itu.
//!
//! Urutan: variabel `COLUMNS` lebih dulu (itu yang benar saat output dialihkan
//! atau saat terminal tidak bisa ditanyai), lalu tanya sistem, lalu menyerah
//! dan biarkan pemanggil memakai nilai bawaan.

use std::env;

pub fn width() -> Option<usize> {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|columns| *columns > 0)
        .or_else(platform_width)
}

#[cfg(windows)]
fn platform_width() -> Option<usize> {
    // CONSOLE_SCREEN_BUFFER_INFO
    #[repr(C)]
    #[derive(Default)]
    struct Coord {
        x: i16,
        y: i16,
    }

    #[repr(C)]
    #[derive(Default)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ScreenBufferInfo {
        size: Coord,
        cursor: Coord,
        attributes: u16,
        window: SmallRect,
        maximum_window_size: Coord,
    }

    const STD_OUTPUT_HANDLE: u32 = 0xffff_fff5;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(handle: u32) -> *mut core::ffi::c_void;
        fn GetConsoleScreenBufferInfo(
            handle: *mut core::ffi::c_void,
            info: *mut ScreenBufferInfo,
        ) -> i32;
    }

    // SAFETY: memanggil dua API konsol Win32 dengan struct ber-`repr(C)` yang
    // tata letaknya sama dengan definisi CONSOLE_SCREEN_BUFFER_INFO. Handle
    // keluaran standar tidak dimiliki dan tidak dibebaskan di sini.
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() {
            return None;
        }
        let mut info = ScreenBufferInfo::default();
        if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
            return None;
        }
        let columns = info.window.right - info.window.left + 1;
        (columns > 0).then_some(columns as usize)
    }
}

#[cfg(unix)]
fn platform_width() -> Option<usize> {
    #[repr(C)]
    #[derive(Default)]
    struct WinSize {
        rows: u16,
        columns: u16,
        x_pixels: u16,
        y_pixels: u16,
    }

    // TIOCGWINSZ berbeda antar sistem.
    #[cfg(target_os = "linux")]
    const TIOCGWINSZ: u64 = 0x5413;
    #[cfg(not(target_os = "linux"))]
    const TIOCGWINSZ: u64 = 0x4008_7468;

    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }

    // SAFETY: `ioctl` diisi penunjuk ke WinSize milik kita sendiri; kernel
    // hanya menulis ke dalamnya saat permintaannya TIOCGWINSZ.
    unsafe {
        let mut size = WinSize::default();
        if ioctl(1, TIOCGWINSZ, &mut size) != 0 {
            return None;
        }
        (size.columns > 0).then_some(size.columns as usize)
    }
}

#[cfg(not(any(windows, unix)))]
fn platform_width() -> Option<usize> {
    None
}
