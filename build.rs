//! Lint saat build.
//!
//! Tugasnya satu, tapi penting: memastikan tidak mungkin kita meng-compile
//! binary yang memakai glyph di luar keluarga yang diizinkan, atau glyph yang
//! namanya tidak ada di `assets/glyphs.toml`.
//!
//! Ini sengaja berupa pemindaian teks yang sederhana, bukan parser TOML penuh.
//! Pemeriksaan yang mendalam (duplikat ekstensi, warna tak dikenal, dan
//! sebagainya) ada di `cargo test` dan di `ikon --audit`. Yang di sini adalah
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

fn main() {
    println!("cargo:rerun-if-changed=icons.toml");
    println!("cargo:rerun-if-changed=assets/glyphs.toml");

    let icons = fs::read_to_string("icons.toml").expect("tidak bisa membaca icons.toml");
    let glyphs_src =
        fs::read_to_string("assets/glyphs.toml").expect("tidak bisa membaca assets/glyphs.toml");

    // Kunci di assets/glyphs.toml, mis: "nf-md-folder_outline" = 0xf0256
    let available: BTreeSet<&str> = glyphs_src
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|rest| rest.split('"').next())
        .collect();

    let mut errors: Vec<String> = Vec::new();
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
