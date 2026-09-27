//! Gerbang keamanan jalur peta netral.
//!
//! `SECURITY.md` menyebut "batas muat data" sebagai kontrol: data pemetaan
//! ditolak bila memuat karakter kontrol/bidi, karena satu karakter ESC cukup
//! untuk menyuntikkan sekuens ke terminal yang menjalankan `ticon`.
//!
//! Tes ini ada karena jalur pemuatan kedua (`--icons-map`, lewat
//! `peta::rules_dari_json`) ditambahkan belakangan. Kontrol yang ada di
//! `Rules::load_from` hanya berlaku untuk `icons.toml`, jadi jalur baru bisa
//! melewatinya. Setiap string dari peta JSON diperiksa dengan aturan yang sama.

use ticon::peta::{rules_dari_berkas, rules_dari_json};
use ticon::render::is_terminal_safe;

/// Bungkus satu aturan jadi peta lengkap supaya tiap kasus bisa fokus pada
/// satu string yang berbahaya.
fn peta(isi: &str) -> String {
    format!(
        r#"{{"schema":"ticon-map/2","families":{{"kerja":"green"}},"defaults":{{
          "file":{{"glyph":"nf-md-file_outline","family":"kerja"}},
          "dir":{{"glyph":"nf-md-folder_outline","family":"kerja"}}}},"rules":[{isi}]}}"#
    )
}

/// Peta dengan satu keluarga bernama `keluarga`, dipakai oleh `families`,
/// aturan, **dan** `defaults`.
///
/// Ketiganya harus konsisten: kalau `defaults` masih menunjuk keluarga lain,
/// peta ditolak karena "keluarga tidak ada" — dan tesnya lulus karena alasan
/// yang salah, bukan karena gerbang yang diuji.
fn peta_dengan_keluarga(keluarga: &str, isi: &str) -> String {
    format!(
        r#"{{"schema":"ticon-map/2","families":{{"{keluarga}":"green"}},"defaults":{{
          "file":{{"glyph":"nf-md-file_outline","family":"{keluarga}"}},
          "dir":{{"glyph":"nf-md-folder_outline","family":"{keluarga}"}}}},"rules":[{isi}]}}"#
    )
}

/// Bukti injeksi terminal lewat jalur peta netral: karakter yang tidak boleh
/// dicetak harus **ditolak saat muat**, bukan diteruskan ke output.
#[test]
fn karakter_kontrol_dalam_kunci_ditolak() {
    let teks = peta(
        r#"{"kind":"ext","key":".rs\u001b[2J","family":"kerja","glyph":"nf-md-language_rust"}"#,
    );
    assert!(
        rules_dari_json(&teks).is_err(),
        "ESC di `key` harus ditolak saat muat, bukan dilewati ke output"
    );
}

#[test]
fn karakter_bidi_dalam_kunci_folder_ditolak() {
    let teks =
        peta(r#"{"kind":"dir","key":"src\u202e","family":"kerja","glyph":"nf-md-folder_outline"}"#);
    assert!(
        rules_dari_json(&teks).is_err(),
        "U+202E di kunci folder harus ditolak: itu membalik tampilan"
    );
}

#[test]
fn karakter_kontrol_dalam_nama_keluarga_ditolak() {
    // Keluarga berbahaya harus benar-benar dipakai di `families`, aturan, dan
    // `defaults` — kalau tidak, peta ditolak karena "keluarga tidak ada" dan
    // tesnya lulus karena alasan yang salah.
    let teks = peta_dengan_keluarga(
        "kerja\u{0007}",
        r#"{"kind":"ext","key":".py","family":"kerja\u0007","glyph":"nf-md-language_rust"}"#,
    );
    let Err(pesan) = rules_dari_json(&teks) else {
        panic!("BEL di nama keluarga harus ditolak");
    };
    assert!(
        pesan.contains("karakter kontrol"),
        "penolakan harus datang dari gerbang karakter kontrol: {pesan}"
    );
}

#[test]
fn karakter_kontrol_dalam_fallback_ditolak() {
    let teks = String::from(
        r#"{"schema":"ticon-map/2","families":{"kerja":"green"},"defaults":{
          "file":{"glyph":"nf-md-file_outline","family":"kerja","fallback":"F\u001b[31m"},
          "dir":{"glyph":"nf-md-folder_outline","family":"kerja"}},
          "rules":[{"kind":"ext","key":".py","family":"kerja","glyph":"nf-md-language_rust"}]}"#,
    );
    // Pesannya ikut diperiksa, bukan cuma statusnya: itu yang mengikat tes ke
    // gerbang karakter kontrol, bukan ke penolakan lain yang kebetulan bekerja.
    let Err(pesan) = rules_dari_json(&teks) else {
        panic!("ESC di `fallback` harus ditolak: nilainya ikut ke ekspor dan output");
    };
    assert!(
        pesan.contains("karakter kontrol"),
        "penolakan harus datang dari gerbang karakter kontrol: {pesan}"
    );
}

/// Sanitasi untuk pesan galat juga harus berlaku: kalau pesan galat sendiri
/// memuat ESC mentah, ia jadi vektor serangan kedua.
#[test]
fn pesan_galat_tidak_memuat_karakter_kontrol_mentah() {
    let teks = peta(
        r#"{"kind":"ext","key":".rs\u001b[2J","family":"kerja","glyph":"nf-md-language_rust"}"#,
    );
    let Err(pesan) = rules_dari_json(&teks) else {
        panic!("peta dengan ESC harus ditolak");
    };
    assert!(
        is_terminal_safe(&pesan),
        "pesan galat sendiri harus aman dicetak: {pesan:?}"
    );
}

/// Peta bersih tetap harus bisa dipakai — gerbang baru jangan sampai menolak
/// data yang sah.
#[test]
fn peta_bersih_tetap_diterima() {
    let teks = peta(
        r#"{"kind":"ext","key":".py","family":"kerja","glyph":"nf-md-language_rust","fallback":"py"}"#,
    );
    let rules = rules_dari_json(&teks).expect("peta bersih harus diterima");
    assert_eq!(rules.resolve_file("a.py").glyph, "nf-md-language_rust");
}

/// Kunci kembar: dua aturan dengan `kind` + `key` sama membuat "mana yang
/// menang?" tidak terjawab, jadi ditolak daripada memilih yang pertama diam-diam.
#[test]
fn kunci_aturan_kembar_ditolak() {
    let teks = String::from(
        r#"{"schema":"ticon-map/2","families":{"kerja":"green"},"defaults":{
          "file":{"glyph":"nf-md-file_outline","family":"kerja"},
          "dir":{"glyph":"nf-md-folder_outline","family":"kerja"}},"rules":[
            {"kind":"ext","key":".py","family":"kerja","glyph":"nf-md-language_rust"},
            {"kind":"ext","key":".py","family":"kerja","glyph":"nf-md-file_outline"}
          ]}"#,
    );
    let Err(pesan) = rules_dari_json(&teks) else {
        panic!("dua aturan dengan kunci sama harus ditolak");
    };
    assert!(pesan.contains("sudah dipakai"), "{pesan}");
}

/// BOM di depan dilewati: peta yang isinya sah tidak boleh gagal dimuat hanya
/// karena tiga byte di depan, dan peta tulis-tangan dari editor Windows sering
/// memakainya.
#[test]
fn bom_di_awal_dilewati() {
    let teks = format!(
        "\u{feff}{}",
        peta(r#"{"kind":"ext","key":".py","family":"kerja","glyph":"nf-md-language_rust"}"#)
    );
    let rules = rules_dari_json(&teks).expect("BOM tidak boleh menggagalkan peta yang sah");
    assert_eq!(rules.resolve_file("a.py").glyph, "nf-md-language_rust");
}

/// Batas ukuran: peta yang terlalu besar ditolak. Pemeriksaan berjalan
/// berdasarkan metadata berkas, jadi tidak perlu memuat isinya dulu.

#[test]
fn peta_terlalu_besar_ditolak() {
    let dir = std::env::temp_dir().join("ikon-keamanan-besar");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("folder sementara");
    let besar = dir.join("besar.json");

    // Satu aturan dengan `key` raksasa: berkas uji tetap JSON yang sah, jadi
    // yang diuji benar-benar batasnya, bukan penolakan sintaks.
    let kunci = format!(".{}", "a".repeat(ticon::peta::MAKS_BERKAS as usize + 1024));
    let isi = format!(
        r#"{{"schema":"ticon-map/2","families":{{"kerja":"green"}},"defaults":{{
           "file":{{"glyph":"nf-md-file_outline","family":"kerja"}},
           "dir":{{"glyph":"nf-md-folder_outline","family":"kerja"}}}},
           "rules":[{{"kind":"ext","key":"{kunci}","family":"kerja","glyph":"nf-md-language_rust"}}]}}"#
    );
    std::fs::write(&besar, &isi).expect("berkas uji ditulis");
    let ukuran = std::fs::metadata(&besar).map(|m| m.len()).unwrap_or(0);
    assert!(
        ukuran > ticon::peta::MAKS_BERKAS,
        "berkas uji harus melewati batas"
    );

    let hasil = rules_dari_berkas(&besar);
    let _ = std::fs::remove_dir_all(&dir);

    let Err(pesan) = hasil else {
        panic!("peta {ukuran} byte harus ditolak");
    };
    assert!(pesan.contains("terlalu besar"), "{pesan}");
}
