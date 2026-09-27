//! Tabel nama glyph -> karakter.
//!
//! Isinya adalah `assets/glyphs.toml` hasil `tools/gen-glyphs.py`: hanya glyph
//! yang benar-benar dipakai, codepoint-nya berasal dari data resmi Nerd Fonts.
//! Karena di-`include_str!`, isinya masuk ke binary — tidak ada pembacaan
//! berkas atau parsing saat dijalankan.

use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
struct GlyphsFile {
    glyphs: BTreeMap<String, i64>,
}

/// Tabel nama glyph -> karakter, hasil `assets/glyphs.toml` yang ikut tertanam
/// di biner.
pub struct Glyphs(BTreeMap<String, char>);

impl Glyphs {
    /// Muat tabel glyph yang tertanam. `assets/glyphs.toml` sudah divalidasi
    /// `build.rs`, jadi kegagalan di sini berarti data yang tertanam rusak —
    /// kondisi yang tidak bisa diperbaiki dari luar, makanya `panic`.
    pub fn bundled() -> Self {
        let parsed: GlyphsFile = toml::from_str(include_str!("../assets/glyphs.toml"))
            .expect("assets/glyphs.toml tidak bisa dibaca — jalankan `python tools/gen-glyphs.py`");

        let map = parsed
            .glyphs
            .into_iter()
            .map(|(name, code)| match char::from_u32(code as u32) {
                // Glyph dicetak mentah ke terminal; codepoint berupa karakter
                // kontrol (ESC dsb) akan dieksekusi, bukan ditampilkan.
                Some(character) if crate::render::is_safe_char(character) => (name, character),
                Some(_) => panic!(
                    "codepoint 0x{code:x} ({name}) berupa karakter kontrol — tidak aman dicetak"
                ),
                None => panic!("codepoint 0x{code:x} bukan karakter unicode yang sah: {name}"),
            })
            .collect();

        Self(map)
    }

    /// Karakter untuk satu nama glyph, atau `None` kalau namanya tidak dikenal.
    pub fn get(&self, name: &str) -> Option<char> {
        self.0.get(name).copied()
    }

    /// Semua nama glyph, terurut.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    /// Semua karakter glyph, terurut.
    pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
        self.0.values().copied()
    }

    /// Jumlah glyph yang tertanam.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True kalau tidak ada glyph sama sekali (tidak terjadi pada data yang
    /// sudah divalidasi `build.rs`).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
