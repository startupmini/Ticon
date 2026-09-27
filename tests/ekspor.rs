//! Kontrak ekspor: JSON yang keluar harus benar-benar bisa dibaca, utuh, dan
//! bolak-balik dengan `--icons-map`.
//!
//! Tes konformansi (`tests/konformasi.rs`) memeriksa *resolusi per nama*; tes
//! ini memeriksa *bentuk keluarannya*. Keduanya saling menutup: ada bug yang
//! hanya salah di JSON (mis. `codepoint` tercetak sebagai karakter) dan tidak
//! akan pernah terlihat dari korpus.

use ticon::glyph::Glyphs;
use ticon::json::{self, Nilai};
use ticon::mapping::Rules;
use ticon::{ekspor_json, ekspor_tsv};

fn rules() -> Rules {
    Rules::load().expect("icons.toml harus bisa dibaca")
}

/// Peta netral hasil ekspor, diurai dengan parser milik crate sendiri — jadi
/// tes ini sekaligus membuktikan parser itu bisa membaca keluarannya sendiri.
fn peta() -> Nilai {
    json::urai(&ekspor_json(&rules(), &Glyphs::bundled())).expect("ekspor JSON harus bisa diurai")
}

#[test]
fn json_lengkap_dan_konsisten() {
    let akar = peta();

    assert_eq!(
        akar.ambil("schema").and_then(Nilai::teks),
        Some(ticon::peta::SKEMA),
        "id skema harus yang diminta"
    );
    assert!(
        akar.ambil("$schema").is_some(),
        "$schema harus ada supaya editor bisa memvalidasi"
    );
    assert_eq!(
        akar.ambil("width_default").and_then(Nilai::bulat),
        Some(1),
        "lebar bawaan harus 1: glyph Nerd Font sering satu sel, dan angka ini \
         hanya petunjuk, bukan hasil pengukuran"
    );

    // `order` memuat keempat tahap pencocokan — dan tidak lebih.
    let order = akar
        .ambil("order")
        .and_then(Nilai::array)
        .expect("order")
        .iter()
        .filter_map(Nilai::teks)
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        vec!["name", "suffix", "prefix", "ext"],
        "urutan kontrak"
    );

    let rules = akar.ambil("rules").and_then(Nilai::array).expect("rules");
    assert!(
        rules.len() > 400,
        "peta ikut ter-ekspor: {} aturan",
        rules.len()
    );

    let mut per_prioritas = std::collections::BTreeMap::new();
    for (i, rule) in rules.iter().enumerate() {
        let posisi = format!("rules[{i}]");
        let kind = rule
            .ambil("kind")
            .and_then(Nilai::teks)
            .unwrap_or_else(|| panic!("{posisi}: kind wajib ada"));
        let key = rule
            .ambil("key")
            .and_then(Nilai::teks)
            .unwrap_or_else(|| panic!("{posisi}: key wajib ada"));
        assert!(!key.is_empty(), "{posisi}: key tidak boleh kosong");
        for wajib in ["family", "color", "glyph"] {
            let nilai = rule
                .ambil(wajib)
                .and_then(Nilai::teks)
                .unwrap_or_else(|| panic!("{posisi}: {wajib} wajib ada"));
            assert!(!nilai.is_empty(), "{posisi}: {wajib} tidak boleh kosong");
        }
        if kind != "dir" {
            let prioritas = rule
                .ambil("priority")
                .and_then(Nilai::bulat)
                .unwrap_or_else(|| panic!("{posisi}: priority wajib ada"));
            let tahap = ticon::mapping::Prioritas::dari_kind(kind).expect("kind dikenal");
            assert_eq!(prioritas, i64::from(tahap.angka()), "{posisi}");
            if kind == "ext" {
                assert!(key.starts_with('.'), "{posisi}: ext harus diawali titik");
            }
            per_prioritas
                .entry(prioritas)
                .or_insert_with(Vec::new)
                .push(key.to_string());
        }
        // Tidak ada nilai sia-sia: `null` berarti tidak ada, jadi jangan ditulis.
        for opsional in ["width", "fallback"] {
            if let Some(isi) = rule.ambil(opsional) {
                assert!(
                    !matches!(isi, Nilai::Null),
                    "{posisi}: {opsional} null tidak ada gunanya"
                );
            }
        }
    }

    // Tidak ada dua aturan dengan kunci kembar pada tahap yang sama.
    for (prioritas, kunci) in per_prioritas {
        let mut unik = kunci;
        unik.sort();
        let total = unik.len();
        unik.dedup();
        assert_eq!(total, unik.len(), "priority {prioritas} punya kunci kembar");
    }
}

#[test]
fn semua_glyph_punya_codepoint() {
    let akar = peta();
    let glyphs = Glyphs::bundled();
    let rules = akar.ambil("rules").and_then(Nilai::array).expect("rules");
    for rule in rules {
        let glyph = rule.ambil("glyph").and_then(Nilai::teks).expect("glyph");
        let cp = rule.ambil("codepoint").and_then(Nilai::bulat);
        let diharapkan = glyphs.get(glyph).map(|c| c as i64);
        assert_eq!(cp, diharapkan, "codepoint untuk {glyph} tidak cocok");
    }
}

#[test]
fn json_dan_tsv_memiliki_isi_yang_sama() {
    let json = peta();
    let tsv = ekspor_tsv(&rules(), &Glyphs::bundled());
    let rules = json.ambil("rules").and_then(Nilai::array).expect("rules");
    let baris_tsv: Vec<&str> = tsv.lines().skip(1).filter(|l| !l.is_empty()).collect();
    // TSV menulis satu baris per keluarga warna lebih dulu; sisanya aturan.
    let keluarga = baris_tsv.len().saturating_sub(rules.len());
    assert_eq!(keluarga, 8, "TSV punya 8 baris keluarga warna");
    for (rule, baris) in rules.iter().zip(baris_tsv.iter().skip(keluarga)) {
        let kolom: Vec<&str> = baris.split('\t').collect();
        let key = rule.ambil("key").and_then(Nilai::teks).expect("key");
        let glyph = rule.ambil("glyph").and_then(Nilai::teks).expect("glyph");
        assert_eq!(kolom[1], key, "kunci TSV harus sama dengan JSON");
        assert_eq!(kolom[2], glyph, "glyph TSV harus sama dengan JSON");
    }
}

#[test]
fn ekspor_bolak_balik_tidak_mengubah_isi() {
    let glyphs = Glyphs::bundled();
    let pertama = ekspor_json(&rules(), &glyphs);

    // Muat kembali lewat jalur yang sama dengan `--icons-map`, lalu ekspor lagi.
    let dimuat = ticon::peta::rules_dari_json(&pertama).expect("hasil ekspor harus bisa dimuat");
    let kedua = ekspor_json(&dimuat, &glyphs);

    // Yang dibandingkan adalah *isi*, bukan urutan baris: saat dimuat, aturan
    // dikelompokkan ulang per kategori (pasangan keluarga + glyph), jadi
    // urutannya boleh berubah. Yang tidak boleh berubah ialah isi setiap aturan
    // dan metadata di luarnya — kalau ada yang hilang di tengah, perbandingan
    // di bawah gagal.
    assert_eq!(
        tanpa_aturan(&pertama),
        tanpa_aturan(&kedua),
        "metadata peta berubah setelah bolak-balik"
    );
    assert_eq!(
        aturan_terurut(&pertama),
        aturan_terurut(&kedua),
        "isi aturan berubah setelah bolak-balik: ada informasi yang hilang"
    );
}

/// Metadata peta: semua baris kecuali isi array `rules`.
fn tanpa_aturan(teks: &str) -> Vec<String> {
    let mut keluar: Vec<String> = teks
        .lines()
        .filter(|l| !l.contains("\"kind\":"))
        .map(|l| l.trim().to_string())
        .collect();
    keluar.sort();
    keluar
}

/// Baris-baris aturan, diurutkan supaya perbandingan tidak bergantung urutan.
fn aturan_terurut(teks: &str) -> Vec<String> {
    let mut keluar: Vec<String> = teks
        .lines()
        .filter(|l| l.contains("\"kind\":"))
        .map(|l| l.trim().trim_end_matches(',').to_string())
        .collect();
    keluar.sort();
    keluar
}

#[test]
fn peta_tertulis_tangan_dipakai_tanpa_salah() {
    let teks = r#"{
      "schema": "ticon-map/2",
      "families": { "kerja": "green" },
      "defaults": {
        "file": { "glyph": "nf-md-file_outline", "family": "kerja", "fallback": "F" },
        "dir": { "glyph": "nf-md-folder_outline", "family": "kerja", "fallback": "D" }
      },
      "rules": [
        { "kind": "ext", "priority": 4, "key": ".myp", "family": "kerja",
          "glyph": "nf-md-language_rust", "codepoint": 988695, "fallback": "rs" }
      ]
    }"#;
    let rules = ticon::peta::rules_dari_json(teks).expect("peta tulis-tangan harus dipakai");
    assert_eq!(rules.resolve_file("a.myp").glyph, "nf-md-language_rust");
    assert_eq!(rules.resolve_file("lain").glyph, "nf-md-file_outline");
    assert_eq!(rules.resolve_dir("apa-saja").glyph, "nf-md-folder_outline");
}
