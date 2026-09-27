//! Konformasi: implementasi referensi (Rust) harus menghasilkan tabel yang sama
//! persis dengan `tests/konformasi/harapan.tsv`.
//!
//! Berkas harapan itu dibaca juga oleh `examples/ts/cek.ts`, jadi resolver
//! TypeScript bisa membuktikan dirinya sama tanpa bergantung pada `ticon` saat
//! runtime.
//!
//! Kolomnya: `nama`, `jenis`, `pola`, `warna`, `codepoint` — semuanya nilai
//! dari skema `ticon-map/1`, bukan kalimat.
//!
//! Kalau peta ikon berubah dengan sengaja, regenerasi lalu baca diff-nya:
//!
//! ```text
//! TICON_REGENERASI=1 cargo test --test konformasi
//! ```

use std::fs;
use std::path::PathBuf;

use ticon::glyph::Glyphs;
use ticon::mapping::{MatchedBy, Prioritas, Rules};

const NAMA: &str = include_str!("konformasi/nama.txt");

fn berkas(nama: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("konformasi")
        .join(nama)
}

/// Nilai `kind` dari aturan yang menang, persis seperti ditulis di kontrak
/// `ticon-map/2`, supaya sisi TypeScript cukup membacanya.
fn jenis(m: MatchedBy) -> &'static str {
    match m.prioritas() {
        Some(prioritas) => prioritas.kind(),
        None => match m {
            MatchedBy::WellKnownFolder => "dir",
            _ => Prioritas::Bawaan.kind(),
        },
    }
}

/// Tabel hasil untuk seluruh korpus: satu baris per entri.
fn tabel() -> String {
    let rules = Rules::load().expect("icons.toml harus bisa dibaca");
    let glyphs = Glyphs::bundled();

    let baris: Vec<String> = NAMA
        .lines()
        .map(str::trim)
        .filter(|baris| !baris.is_empty() && !baris.starts_with('#'))
        .map(|entri| {
            let folder = entri.starts_with("dir:");
            let teks = entri.strip_prefix("dir:").unwrap_or(entri);
            let (resolved, penjelasan) = if folder {
                (rules.resolve_dir(teks), rules.explain_dir(teks))
            } else {
                (rules.resolve_file(teks), rules.explain_file(teks))
            };
            let pola = penjelasan
                .winner()
                .map(|k| k.pattern.clone())
                .unwrap_or_default();
            let codepoint = glyphs
                .get(&resolved.glyph)
                .map(|c| format!("{:#x}", c as u32))
                .unwrap_or_else(|| "-".to_string());
            format!(
                "{entri}\t{}\t{pola}\t{}\t{codepoint}",
                jenis(resolved.matched_by),
                resolved.color
            )
        })
        .collect();
    baris.join("\n") + "\n"
}

#[test]
fn resolver_rust_sesuai_korpus() {
    let sekarang = tabel();
    let berkas = berkas("harapan.tsv");

    if std::env::var_os("TICON_REGENERASI").is_some() {
        fs::write(&berkas, &sekarang).expect("harapan.tsv harus bisa ditulis");
        return;
    }

    let harapan = fs::read_to_string(&berkas).unwrap_or_else(|_| {
        panic!(
            "tests/konformasi/harapan.tsv belum ada. Regenerasi dengan \
             TICON_REGENERASI=1 cargo test --test konformasi"
        )
    });
    // End-of-line dinormalkan dulu: checkout di Windows bisa mengubahnya jadi
    // CRLF, dan itu bukan divergensi isi yang mau diuji.
    let harapan = harapan.replace("\r\n", "\n");
    assert_eq!(
        sekarang, harapan,
        "resolver Rust menyimpang dari korpus konformasi. Kalau ini disengaja, \
         regenerasi dengan TICON_REGENERASI=1 lalu periksa diff-nya."
    );
}
