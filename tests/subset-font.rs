//! Subset font untuk jalur render tanpa Nerd Font.
//!
//! `ticon` mencetak karakter, jadi kalau terminal tidak punya Nerd Font,
//! ikon jadi kotak — program tidak bisa mengatasinya, karena itu pertanyaan
//! terminal. Yang bisa diatasi adalah jalur yang me-*raster* sendiri (half-block):
//! di sana font boleh ikut dikemas.
//!
//! Font aslinya 2,2 MB; yang dikemas hanya glyph yang benar dipakai
//! (`raster_data::GLYPH`), jadi sekitar 9 KiB. Tes di bawah menjaga subset itu
//! tetap lengkap: kalau `icons.toml` ditambah glyph baru tanpa regenerasi,
//! tes gagal alih-alih diam-diam menghasilkan kotak.

use ticon::mapping::Rules;
use ticon::raster_data::GLYPH;

/// Semua nama glyph yang dirujuk data pemetaan.
fn glyph_dipakai(rules: &Rules) -> Vec<String> {
    let mut semua: Vec<String> = Vec::new();
    for category in rules.categories.values() {
        semua.push(category.glyph.clone());
        semua.extend(category.by_ext.values().cloned());
    }
    for rule in rules.dirs.values() {
        semua.push(rule.glyph.clone());
    }
    semua.sort();
    semua.dedup();
    semua
}

#[test]
fn subset_font_mencakup_semua_glyph_yang_dipakai() {
    let rules = Rules::load().expect("icons.toml harus bisa dibaca");
    let dipakai = glyph_dipakai(&rules);
    let ada: Vec<&str> = GLYPH.iter().map(|(nama, _)| *nama).collect();

    let kurang: Vec<&String> = dipakai
        .iter()
        .filter(|n| !ada.contains(&n.as_str()))
        .collect();
    assert!(
        kurang.is_empty(),
        "glyph ini dipakai icons.toml tapi tidak ada di assets/ticon-icons.ttf: {kurang:?}. \
         Jalankan `python tools/gen-font.py`"
    );
}

#[test]
fn subset_font_tidak_membawa_glyph_yang_tidak_dipakai() {
    let rules = Rules::load().expect("icons.toml harus bisa dibaca");
    let dipakai = glyph_dipakai(&rules);
    for (nama, _) in GLYPH {
        assert!(
            dipakai.iter().any(|n| n == nama),
            "`{nama}` ada di subset tapi tidak dipakai icons.toml — subsetnya bisa dikecilkan"
        );
    }
}

#[test]
fn codepoint_subset_konsisten_dengan_tabel_glyph() {
    let glyphs = ticon::glyph::Glyphs::bundled();
    for (nama, codepoint) in GLYPH {
        let isi = glyphs.get(nama);
        assert_eq!(
            isi.map(|c| c as u32),
            Some(*codepoint),
            "codepoint `{nama}` di subset tidak cocok dengan assets/glyphs.toml"
        );
    }
}
