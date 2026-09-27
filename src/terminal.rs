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

/// Lebar terminal, dalam sel: variabel `COLUMNS` dulu, lalu tanya sistem.
/// `None` kalau tidak bisa diketahui — pemanggil memakai nilai bawaannya.
pub fn width() -> Option<usize> {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|columns| *columns > 0)
        .or_else(platform_width)
}

// Struktur Win32 yang dipakai di bawah. Ditaruh di luar fungsi supaya
// `#[cfg(test)]` bisa menguji ukurannya: layout yang salah = baca angka acak.

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct Coord {
    x: i16,
    y: i16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct SmallRect {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct ScreenBufferInfo {
    size: Coord,
    cursor: Coord,
    attributes: u16,
    window: SmallRect,
    maximum_window_size: Coord,
}

#[cfg(windows)]
fn platform_width() -> Option<usize> {
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
        // Kegagalan Win32 mengembalikan INVALID_HANDLE_VALUE (-1), bukan NULL.
        if handle.is_null() || handle as isize == -1 {
            return None;
        }
        let mut info = ScreenBufferInfo::default();
        if GetConsoleScreenBufferInfo(handle, &mut info) == 0 {
            return None;
        }
        // Lebar dihitung dalam i32: `right - left + 1` bisa melewati batas i16
        // pada buffer yang sangat lebar, dan overflow i16 akan panic di build
        // debug sebelum sempat difilter.
        let columns = i32::from(info.window.right) - i32::from(info.window.left) + 1;
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    /// `GetConsoleScreenBufferInfo` menulis ke struct ini; urutan atau lebar
    /// field yang salah = membaca angka acak tanpa error kompilasi. Tes ini
    /// yang menjaganya. Hanya dijalankan di Windows — dan itulah sebabnya CI
    /// sekarang memakai matrix ubuntu + windows.
    #[test]
    fn layout_screen_buffer_info_sesuai_abi_win32() {
        assert_eq!(size_of::<Coord>(), 4);
        assert_eq!(size_of::<SmallRect>(), 8);
        // 2 COORD (8) + WORD (2) + SMALL_RECT (8) + COORD (4) = 22
        assert_eq!(size_of::<ScreenBufferInfo>(), 22);
        assert_eq!(align_of::<ScreenBufferInfo>(), 2);
    }
}
