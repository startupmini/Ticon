//! Lint saat build.
//!
//! Tugasnya satu, tapi penting: memastikan tidak mungkin kita meng-compile
//! binary yang memakai glyph di luar keluarga yang diizinkan, atau glyph yang
//! namanya tidak ada di `assets/glyphs.toml`.
//!
//! Ini sengaja berupa pemindaian teks yang sederhana, bukan parser TOML penuh.
//! Pemeriksaan yang mendalam (duplikat ekstensi, warna tak dikenal, dan
//! sebagainya) ada di `cargo test` dan di `ticon --audit`. Yang di sini adalah
//! jaring paling murah untuk kesalahan paling sering: salah ketik nama glyph.
//!
//! Alasan aturan keluarga glyph ditegakkan: konsistensi visual. Semua ikon
//! `nf-md-` digambar di grid 24px yang sama dengan stroke seragam, itulah yang
//! membuat setnya terasa flat. Satu glyph dari keluarga lain sudah cukup untuk
//! merusaknya.

use std::collections::BTreeSet;
use std::fs;

/// Keluarga glyph yang boleh dipakai. Tambahkan dengan sadar, bukan karena
/// "kebetulan ada".
const ALLOWED_FAMILIES: &[&str] = &["nf-md-", "nf-oct-"];

/// Karakter yang aman dicetak ke terminal. Duplikasi dari `src/render.rs`
/// sebab build script adalah crate terpisah — sama seperti ALLOWED_FAMILIES.
fn is_terminal_safe(c: char) -> bool {
    !matches!(
        c,
        '\u{0}'..='\u{1f}'   // C0 termasuk ESC
            | '\u{7f}'..='\u{9f}' // DEL + C1
            | '\u{61c}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202a}'..='\u{202e}' // pengendali arah bidi
            | '\u{2066}'..='\u{2069}' // isolasi bidi
    )
}

fn main() {
    println!("cargo:rerun-if-changed=icons.toml");
    println!("cargo:rerun-if-changed=assets/glyphs.toml");

    let icons = fs::read_to_string("icons.toml").expect("tidak bisa membaca icons.toml");
    let glyphs_src =
        fs::read_to_string("assets/glyphs.toml").expect("tidak bisa membaca assets/glyphs.toml");

    let mut errors: Vec<String> = Vec::new();
    let mut available: BTreeSet<&str> = BTreeSet::new();

    // Sekaligus verifikasi tiap codepoint: glyph dicetak mentah oleh
    // `--list`/`--gallery`, jadi karakter kontrol di tabel = injeksi terminal.
    for line in glyphs_src.lines().map(str::trim) {
        let Some(rest) = line.strip_prefix('"') else {
            continue;
        };
        let Some(akhir) = rest.find('"') else {
            continue;
        };
        let name = &rest[..akhir];
        available.insert(name);

        let Some(nilai) = rest[akhir + 1..].split('=').nth(1) else {
            errors.push(format!(
                "glyph '{name}' tidak punya nilai `= ...` yang terbaca"
            ));
            continue;
        };
        let Some(hex) = nilai.trim().strip_prefix("0x") else {
            errors.push(format!("codepoint glyph '{name}' bukan heksadesimal `0x`"));
            continue;
        };
        match u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
            Some(c) if is_terminal_safe(c) => {}
            Some(_) => errors.push(format!(
                "glyph '{name}' memakai codepoint 0x{hex} yang tidak aman dicetak \
                 (karakter kontrol)"
            )),
            None => errors.push(format!(
                "codepoint 0x{hex} untuk glyph '{name}' tidak terbaca sebagai karakter unicode"
            )),
        }
    }

    let mut used: BTreeSet<String> = BTreeSet::new();

    for (idx, _) in icons.match_indices("\"nf-") {
        let rest = &icons[idx + 1..];
        let end = rest.find('"').unwrap_or(rest.len());
        let name = &rest[..end];
        used.insert(name.to_string());

        if !ALLOWED_FAMILIES
            .iter()
            .any(|family| name.starts_with(family))
        {
            errors.push(format!(
                "glyph '{name}' bukan dari keluarga yang diizinkan (hanya {})",
                ALLOWED_FAMILIES.join(", ")
            ));
        }
        if !available.contains(name) {
            errors.push(format!("glyph '{name}' tidak ada di assets/glyphs.toml"));
        }
    }

    // Glyph yang ada di tabel tapi tidak dipakai = bobot mati. Peringatkan saja.
    for name in available.iter().filter(|name| !used.contains(**name)) {
        println!("cargo:warning=glyph '{name}' ada di tabel tapi tidak dipakai siapa pun");
    }

    if !errors.is_empty() {
        panic!(
            "\nmapping ikon tidak valid:\n  - {}\n\nPerbaiki icons.toml, atau regenerasi tabel \
             glyph dengan `python tools/gen-glyphs.py`.\n",
            errors.join("\n  - ")
        );
    }
}
