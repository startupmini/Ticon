//! Alfa repo harus satu: huruf Latin.
//!
//! Simbol (tanda hubung, panah, blok, akar kuadrat) tetap boleh — itu gaya
//! repo sejak awal. Yang dilarang adalah HURUF dari alfabet lain: CJK, Kana,
//! Hangul, Cyrillic, Greek, Arab, Ibrani, Devanagari, dan sejenisnya. Huruf
//! seperti itu hampir selalu berarti ada yang mengetik dari sumber yang salah
//! (salin-tempel dari keluaran mesin atau dokumen lain), dan di dokumentasi
//! kode ia langsung tak terbaca — di nama berkas ia bisa merusak perintah.
//!
//! Nama berkas ikut diperiksa, bukan hanya isinya.

use std::fs;
use std::path::{Path, PathBuf};

/// Direktori yang tidak dibaca: build output, cache, dan skrip/tooling.
const LEWATI: &[&str] = &["target", ".git", ".cache", "__pycache__", ".freebuff"];

fn latin(ch: char) -> bool {
    matches!(ch,
        'A'..='Z'
            | 'a'..='z'
            | '\u{00AA}'..='\u{00BA}'   // ordinal & maskula
            | '\u{00C0}'..='\u{024F}'   // Latin-1 Supplement + Latin Extended A/B
            | '\u{1E00}'..='\u{1EFF}'   // Latin Extended Additional
            | '\u{2C60}'..='\u{2C7F}'   // Latin Extended-C
            | '\u{A720}'..='\u{A7FF}'   // Latin Extended-D
            | '\u{AB30}'..='\u{AB6F}'   // Latin Extended-E
            | '\u{FB00}'..='\u{FB06}'   // ligatur Latin
            | '\u{FF21}'..='\u{FF3A}'   // Latin penuh-lebar
            | '\u{FF41}'..='\u{FF5A}'
    )
}

fn huruf_asing(ch: char) -> bool {
    ch.is_alphabetic() && !latin(ch)
}

/// Nama skrip sederhana untuk pesan galat — std tidak menyediakan nama skrip.
fn nama_skrip(ch: char) -> &'static str {
    match ch as u32 {
        0x3040..=0x30FF | 0x31F0..=0x31FF => "Kana (Jepang)",
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => "Han (CJK)",
        0xAC00..=0xD7AF | 0x1100..=0x11FF => "Hangul (Korea)",
        0x0400..=0x04FF => "Cyrillic",
        0x0370..=0x03FF => "Greek",
        0x0590..=0x05FF => "Ibrani",
        0x0600..=0x06FF => "Arab",
        0x0900..=0x097F => "Devanagari",
        0x0E00..=0x0E7F => "Thai",
        0x10A0..=0x10FF => "Georgian",
        _ => "skrip lain",
    }
}

fn kumpulkan(dir: &Path, akar: &Path, temuan: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let nama = entry.file_name();
        let nama = nama.to_string_lossy();

        if path.is_dir() {
            if !LEWATI.iter().any(|lewati| nama == *lewati) {
                kumpulkan(&path, akar, temuan);
            }
            continue;
        }

        // Nama berkas pun harus Latin: nama asing merusak perintah dan diff.
        for ch in nama.chars().filter(|c| huruf_asing(*c)) {
            temuan.push(format!(
                "{}: nama berkas memuat huruf '{}' (U+{:04X}, {})",
                path.display(),
                ch,
                ch as u32,
                nama_skrip(ch)
            ));
        }

        // Berkas yang bukan UTF-8 (ikon biner) dilewati diam-diam.
        let Ok(teks) = fs::read_to_string(&path) else {
            continue;
        };
        for (baris, isi) in teks.lines().enumerate() {
            for ch in isi.chars().filter(|c| huruf_asing(*c)) {
                temuan.push(format!(
                    "{}:{}: huruf '{}' (U+{:04X}, {})",
                    path.strip_prefix(akar).unwrap_or(&path).display(),
                    baris + 1,
                    ch,
                    ch as u32,
                    nama_skrip(ch)
                ));
            }
        }
    }
}

#[test]
fn repo_tetap_satu_alfabet_latin() {
    let akar = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut temuan = Vec::new();
    kumpulkan(&akar, &akar, &mut temuan);

    // Folder .cache sengaja dilewati: isinya salinan data upstream, bukan
    // bagian repo, dan bisa sebesar itu.
    temuan.sort();
    temuan.dedup();
    let contoh: Vec<String> = temuan.iter().take(20).cloned().collect();
    assert!(
        temuan.is_empty(),
        "ditemukan {} huruf non-Latin di repo:\n  {}",
        temuan.len(),
        contoh.join("\n  ")
    );
}

/// Pratinjau ikon di README harus selalu cocok dengan pemetaan hari ini.
///
/// Pratinjau itu berisi glyph Nerd Font asli di dalam `preview.svg`, jadi ia
/// bisa basi diam-diam kalau data berubah: gambar masih menunjuk ikon lama
/// tanpa ada yang salah. Tes ini menutupnya — kalau gagal, jalankan
/// `python tools/gen-preview.py` lalu periksa bedanya.
#[test]
fn pratinjau_readme_sesuai_pemetaan() {
    let svg = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("preview.svg"))
        .expect("preview.svg harus ada; jalankan `python tools/gen-preview.py`");

    // Isi pratinjau adalah keluaran `ticon` asli: satu ikon + satu nama per baris.
    let baris: Vec<String> = svg
        .lines()
        .filter_map(|l| l.trim().strip_prefix("<text x="))
        .filter_map(|s| s.split_once('>').map(|(_, isi)| isi))
        .map(|isi| isi.split("</text>").next().unwrap_or("").trim().to_string())
        .collect();
    assert!(!baris.is_empty(), "tidak ada baris ikon di preview.svg");

    let rules = ticon::mapping::Rules::load().expect("icons.toml harus bisa dibaca");
    // Pratinjau menyimpan karakter; `Rules` menyimpan nama glyph. Jembatannya
    // tabel glyph bawaan.
    let glyphs = ticon::glyph::Glyphs::bundled();

    for baris in &baris {
        let dipisah: Vec<&str> = baris.split_whitespace().collect();
        assert!(
            dipisah.len() == 2,
            "baris pratinjau harus ikon + nama: {baris}"
        );
        let glyph: char = dipisah[0].chars().next().expect("glyph harus ada");
        let nama = dipisah[1];

        // Nama folder tidak selalu berekstensi, jadi dua-duanya dicoba: kalau
        // salah satu cocok dengan glyph yang tertulis, baris ini benar.
        let cocok = glyphs.get(&rules.resolve_dir(nama).glyph) == Some(glyph)
            || glyphs.get(&rules.resolve_file(nama).glyph) == Some(glyph);
        assert!(
            cocok,
            "pratinjau menyebut glyph U+{:04X} untuk `{nama}`, tapi pemetaan sekarang \
             memakai folder={} / berkas={}",
            glyph as u32,
            rules.resolve_dir(nama).glyph,
            rules.resolve_file(nama).glyph
        );
    }
}
