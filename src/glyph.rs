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

pub struct Glyphs(BTreeMap<String, char>);

impl Glyphs {
    pub fn bundled() -> Self {
        let parsed: GlyphsFile = toml::from_str(include_str!("../assets/glyphs.toml"))
            .expect("assets/glyphs.toml tidak bisa dibaca — jalankan `python tools/gen-glyphs.py`");

        let map = parsed
            .glyphs
            .into_iter()
            .map(|(name, code)| match char::from_u32(code as u32) {
                Some(character) => (name, character),
                None => panic!("codepoint 0x{code:x} bukan karakter unicode yang sah: {name}"),
            })
            .collect();

        Self(map)
    }

    pub fn get(&self, name: &str) -> Option<char> {
        self.0.get(name).copied()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
