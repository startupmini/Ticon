//! Kontrak icon pack: pewarisan/override, validasi, dan penjagaan klaim.
//!
//! Yang paling dijaga di sini adalah hal yang paling mudah rusak diam-diam:
//! **override** (`rules` menimpa, bukan menumpuk) dan **bentuk** (yang dipakai
//! kalau glyph Nerd Font tidak bisa digambar). Kalau salah satu berubah, output
//! `ticon` berubah tanpa ada yang salah kompilasi.

use ticon::glyph::Glyphs;
use ticon::mapping::Rules;
use ticon::pack::{self, Asal, Bentuk, Pack};

fn dasar() -> Rules {
    Rules::load().expect("icons.toml harus bisa dibaca")
}

/// Bungkus isi pack jadi JSON lengkap, supaya tiap kasus uji bisa fokus pada
/// satu hal yang salah.
fn bungkus(isi: &str) -> String {
    format!(
        r#"{{"schema":"{}","pack":{{"name":"uji","version":"1.0.0"}}{isi}}}"#,
        pack::SKEMA
    )
}

fn muat(isi: &str) -> Result<Pack, String> {
    pack::muat_teks(&bungkus(isi), Asal::Berkas("uji".into()), &dasar())
}

#[test]
fn pack_kosong_tetap_menghasilkan_peta_yang_jalur() {
    let pack = muat("").expect("pack tanpa rules harus bisa dipakai");
    let rules = dasar();
    // Peta dasar tidak boleh berubah kalau pack tidak menimpa apa pun.
    assert_eq!(
        pack.rules.resolve_file("main.rs").glyph,
        rules.resolve_file("main.rs").glyph
    );
    assert_eq!(
        pack.rules.resolve_file("catatan.md").glyph,
        rules.resolve_file("catatan.md").glyph
    );
}

#[test]
fn aturan_ekstensi_menimpa_bukan_menumpuk() {
    let pack = muat(
        r#","rules":[{"kind":"ext","key":".rs","family":"kode","glyph":"nf-md-file_outline"}]"#,
    )
    .expect("pack harus bisa dipakai");

    let isi = pack.rules.resolve_file("main.rs");
    assert_eq!(
        isi.glyph, "nf-md-file_outline",
        "ext .rs harus memakai glyph dari pack"
    );
    // Dan tidak boleh ada sisa aturan .rs di kategori lama: kalau ada, kunci
    // itu diklaim dua kali dan artifaknya tidak jelas.
    let sisa: Vec<&String> = pack
        .rules
        .categories
        .values()
        .filter(|kategori| kategori.family == "kode" && kategori.glyph == "nf-md-language_rust")
        .flat_map(|kategori| kategori.ext.iter())
        .filter(|ext| ext.as_str() == ".rs")
        .collect();
    assert!(sisa.is_empty(), ".rs masih diklaim kategori lama: {sisa:?}");
}

#[test]
fn aturan_baru_ikut_berfungsi() {
    let pack = muat(
        r#","rules":[{"kind":"ext","key":".wibble","family":"data","glyph":"nf-md-file_outline"}]"#,
    )
    .expect("pack harus bisa dipakai");
    assert_eq!(
        pack.rules.resolve_file("a.wibble").glyph,
        "nf-md-file_outline"
    );
    // Aturan lama tidak boleh ikut hilang.
    assert_eq!(
        pack.rules.resolve_file("app.test.ts").glyph,
        dasar().resolve_file("app.test.ts").glyph
    );
}

#[test]
fn folder_well_known_bisa_ditimpa() {
    let pack = muat(
        r#","rules":[{"kind":"dir","key":"src","family":"media","glyph":"nf-md-folder_outline"}]"#,
    )
    .expect("pack harus bisa dipakai");
    let isi = pack.rules.resolve_dir("src");
    assert_eq!(isi.color, "magenta", "warna ikut keluarga yang diminta");
    assert!(
        !pack.rules.overlaps.iter().any(|o| o.contains("src")),
        "src tidak boleh jadi kunci yang diklaim dua kali: {:?}",
        pack.rules.overlaps
    );
}

#[test]
fn warna_keluarga_baru_boleh_ditambah_dan_dipakai() {
    let pack = muat(
        r#","families":{"khas":"magenta"},
           "rules":[{"kind":"ext","key":".wibble","family":"khas","glyph":"nf-md-file_outline"}],
           "shapes":{"khas":"*"}"#,
    )
    .expect("pack harus bisa memakai keluarga baru");
    assert_eq!(
        pack.rules.families.get("khas").map(String::as_str),
        Some("magenta")
    );
    // `shapes` boleh menyebut keluarga yang baru ditambah pack yang sama.
    assert_eq!(pack.bentuk.keluarga.get("khas"), Some(&'*'));
    assert_eq!(pack.bentuk.untuk(&pack.rules, "magenta", false), Some('*'));
}

#[test]
fn dua_aturan_kembar_dalam_satu_pack_ditolak() {
    let pesan = muat(
        r#","rules":[
            {"kind":"ext","key":".x","family":"kode","glyph":"nf-md-file_outline"},
            {"kind":"ext","key":".x","family":"data","glyph":"nf-md-file_outline"}
        ]"#,
    )
    .err()
    .expect("aturan kembar ditolak");
    assert!(pesan.contains("diulang"), "{pesan}");
}

#[test]
fn min_ticon_yang_lebih_baru_ditolak() {
    let pesan = pack::muat_teks(
        &format!(
            r#"{{"schema":"{}","pack":{{"name":"uji","version":"1.0.0","min_ticon":"999.0.0"}}}}"#,
            pack::SKEMA
        ),
        Asal::Berkas("uji".into()),
        &dasar(),
    )
    .err()
    .expect("min_ticon ke depan ditolak");
    assert!(pesan.contains("lebih baru"), "{pesan}");

    // Yang sama persis dengan versi yang jalan harus diterima: pack yang
    // ditulis untuk rilis ini tidak boleh ditolak oleh rilis ini.
    pack::muat_teks(
        &format!(
            r#"{{"schema":"{}","pack":{{"name":"uji","version":"1.0.0","min_ticon":"{}"}}}}"#,
            pack::SKEMA,
            env!("CARGO_PKG_VERSION")
        ),
        Asal::Berkas("uji".into()),
        &dasar(),
    )
    .expect("min_ticon yang sama harus diterima");
}

/// Muat teks mentah, untuk kasus yang butuhPack-nya utuh (bukan lewat
/// [`bungkus`]).
fn mentah(teks: &str) -> Result<Pack, String> {
    pack::muat_teks(teks, Asal::Berkas("uji".into()), &dasar())
}

#[test]
fn nilai_yang_ditolak() {
    let kasus: &[(&str, &str, &str)] = &[
        ("kunci level atas asing", r#","ngawur":1"#, "tidak dikenal"),
        (
            "keluarga tak dikenal",
            r#","rules":[{"kind":"ext","key":".x","family":"hantu","glyph":"nf-md-file_outline"}]"#,
            "hantu",
        ),
        (
            "warna tak dikenal",
            r#","families":{"khas":"chartreuse"}"#,
            "chartreuse",
        ),
        (
            "glyph tak dikenal",
            r#","rules":[{"kind":"ext","key":".x","family":"kode","glyph":"nf-md-hantu"}]"#,
            "tidak dikenal",
        ),
        (
            "kind tak dikenal",
            r#","rules":[{"kind":"warna","key":".x","family":"kode","glyph":"nf-md-file_outline"}]"#,
            "warna",
        ),
        (
            "ext tanpa titik",
            r#","rules":[{"kind":"ext","key":"x","family":"kode","glyph":"nf-md-file_outline"}]"#,
            "titik",
        ),
        (
            "folder huruf besar",
            r#","rules":[{"kind":"dir","key":"Src","family":"kode","glyph":"nf-md-folder_outline"}]"#,
            "huruf kecil",
        ),
        (
            "kunci aturan asing",
            r#","rules":[{"kind":"ext","key":".x","family":"kode","glyph":"nf-md-file_outline","warnanya":"hijau"}]"#,
            "tidak dikenal",
        ),
        (
            "bentuk dua karakter",
            r#","shapes":{"kode":"xx"}"#,
            "tepat satu karakter",
        ),
        ("bentuk kosong", r#","shapes":{"kode":""}"#, "kosong"),
        (
            "shapes untuk keluarga tak ada",
            r#","shapes":{"hantu":"*"}"#,
            "hantu",
        ),
    ];

    for (nama, isi, economics) in kasus {
        let pesan = muat(isi)
            .err()
            .unwrap_or_else(|| panic!("{nama} seharusnya ditolak, tapi diterima"));
        assert!(pesan.contains(economics), "{nama}: {pesan}");
    }

    let meta_asing = mentah(&format!(
        r#"{{"schema":"{}","pack":{{"name":"uji","version":"1","ngawur":true}}}}"#,
        pack::SKEMA
    ))
    .err()
    .expect("metadata asing ditolak");
    assert!(meta_asing.contains("tidak dikenal"), "{meta_asing}");

    // Skema yang salah harus menyebut nama yang benar, bukan cuma
    // "tidak dikenal": pack yang ditulis untuk format lain perlu tahu
    // format mana yang dipakai `ticon`.
    let skema = mentah(r#"{"schema":"ticon-map/2","pack":{"name":"uji","version":"1"}}"#)
        .err()
        .expect("skema yang salah ditolak");
    assert!(skema.contains(pack::SKEMA), "{skema}");

    // Metadata ikut dicetak ke terminal, jadi harus lewat gerbang yang sama
    // dengan peta netral.
    let esc = mentah(&format!(
        r#"{{"schema":"{}","pack":{{"name":"uji","version":"1","description":"a\u001b[2Jb"}}}}"#,
        pack::SKEMA
    ))
    .err()
    .expect("karakter kontrol di metadata ditolak");
    assert!(esc.contains("kontrol"), "{esc}");
    // Strikingly: kunci yang Builder-nya Rules::dari_bagian tidak terima
    // (`width`, `fallback`) sengaja TIDAK ada di kontrak pack v1. Kalau
    // diterima diam-diam, pack bisa-half mengatakannya diterapkan padahal
    // tidak.
    let pesan = muat(
        r#","rules":[{"kind":"ext","key":".x","family":"kode",
            "glyph":"nf-md-file_outline","width":2}]"#,
    )
    .err()
    .expect("width tidak dikenal di pack ditolak");
    assert!(pesan.contains("tidak dikenal"), "{pesan}");
}

#[test]
fn pack_bawaan_memuat_shapes_yang_aman() {
    let glyphs = Glyphs::bundled();
    for nama in [pack::NAMA_BAWAAN, pack::NAMA_SEMPIT] {
        let pack = pack::muat_nama(nama, &dasar())
            .unwrap_or_else(|e| panic!("pack bawaan `{nama}` harus bisa dimuat: {e}"));

        assert!(
            !pack.bentuk.is_kosong(),
            "pack `{nama}` harus menyediakan bentuk cadangan"
        );
        for bentuk in pack.bentuk.semua_karakter() {
            let codepoint = bentuk as u32;
            // Tidak boleh karakter kontrol: bentuk masuk ke output terminal.
            assert!(
                !bentuk.is_control(),
                "karakter kontrol U+{codepoint:04X} di pack `{nama}`"
            );
            // Tidak boleh private-use: bentuk yang butuh Nerd Font bukan cadangan.
            assert!(
                !(0xE000..=0xF8FF).contains(&codepoint),
                "pack `{nama}` memakai karakter private-use U+{codepoint:04X}; itu butuh Nerd Font juga"
            );
            // Dan tidak boleh ada glyph Nerd Font yang bocor ke `shapes`.
            assert!(
                !glyphs.chars().any(|c| c == bentuk),
                "karakter `{bentuk}` di pack `{nama}` ternyata juga glyph Nerd Font"
            );
        }
    }
}

#[test]
fn pack_sempit_tidak_ada_karakter_ambiguous() {
    // Jaminan nama: pack yang menjanjikan "satu sel di semua terminal" tidak
    // boleh memuat karakter `East_Asian_Width` A. Daftar di bawah adalah
    // karakter yang pack `shape` memang pakai; kalau suatu saat ada yang
    // memindahkannya ke `sempit`, tes ini yang menangkap.
    let ambigu: Vec<char> = "≡→§●▲□▼•".chars().collect();
    let pack = pack::muat_nama(pack::NAMA_SEMPIT, &dasar()).expect("pack sempit harus dimuat");
    for bentuk in pack.bentuk.semua_karakter() {
        assert!(
            !ambigu.contains(&bentuk),
            "pack `sempit` memakai `{bentuk}` (U+{:04X}) yang ambiguous-width",
            bentuk as u32
        );
    }
}

#[test]
fn bentuk_tidak_pernah_menggantikan_glyph_nerd() {
    // Bentuk ada di field terpisah supaya pack yang tidak menyediakannya
    // tidak mengubah apa pun: `icon_for` harus tetap Nerd Font.
    let rules = dasar();
    let glyphs = Glyphs::bundled();
    let icon = rules.icon_for(&glyphs, "main.rs");
    assert!(icon.shape.is_none(), "tanpa pack, shape harus None");
    assert!(icon.ch.is_some(), "tanpa pack, ch tetap Nerd Font");
}

#[test]
fn icon_dengan_shape_mengisi_shape_saja() {
    let pack = pack::muat_nama(pack::NAMA_BAWAAN, &dasar()).expect("pack shape harus dimuat");
    let glyphs = Glyphs::bundled();
    let icon = pack
        .rules
        .icon_for_dengan_shape(&glyphs, "main.rs", Some(&pack.bentuk));

    assert_eq!(icon.shape, Some('≡'), "kode harus dapat bentuknya");
    assert!(
        icon.ch.is_some(),
        "shape tidak boleh menghapus glyph: pemanggil yang punya font tetap butuh ch"
    );
    let folder = pack
        .rules
        .icon_for_dir_dengan_shape(&glyphs, "src", Some(&pack.bentuk));
    assert_eq!(folder.shape, Some('▼'), "folder dapat bentuk sendiri");
}

#[test]
fn bentuk_kosong_tidak_mengubah_apa_pun() {
    let rules = dasar();
    let glyphs = Glyphs::bundled();
    let kosong = Bentuk::kosong();
    let icon = rules.icon_for_dengan_shape(&glyphs, "main.rs", Some(&kosong));
    assert!(icon.shape.is_none());
    assert_eq!(icon.color, rules.icon_for(&glyphs, "main.rs").color);
}

#[test]
fn pack_bawaan_menang_atas_berkas_dengan_nama_sama() {
    // Ini yang membuat `--icons-pack shape` selalu berarti yang tertanam,
    // bukan `./ticon-packs/shape/pack.json` yang isinya bisa apa saja.
    let pack = pack::muat_nama(pack::NAMA_BAWAAN, &dasar()).expect("pack bawaan harus dimuat");
    assert_eq!(pack.asal, Asal::Bawaan(pack::NAMA_BAWAAN));
    assert_eq!(pack.meta.nama, pack::NAMA_BAWAAN);
}

#[test]
fn pack_tak_dikenal_menyebut_lokasi_yang_dicoba() {
    let pesan = pack::muat_nama("tidak-ada-pack-ini", &dasar())
        .err()
        .expect("pack tak dikenal harus ditolak");
    // Pesannya harus berguna: menyebut lokasi yang dicoba, supaya pengguna
    // tidak perlu menebak ke mana harus menaruh berkasnya.
    assert!(pesan.contains("ticon-packs"), "{pesan}");
}

#[test]
fn daftar_pack_menyertakan_kedua_bawaan() {
    let daftar = pack::daftar();
    let bawaan: Vec<&str> = daftar
        .iter()
        .filter(|info| matches!(info.asal, Asal::Bawaan(_)))
        .map(|info| info.nama.as_str())
        .collect();
    assert!(bawaan.contains(&pack::NAMA_BAWAAN), "{bawaan:?}");
    assert!(bawaan.contains(&pack::NAMA_SEMPIT), "{bawaan:?}");
    for info in &daftar {
        let galat = info.meta.as_ref().err().map(|e| e.as_str());
        assert!(
            galat.is_none(),
            "pack bawaan `{}` harus punya metadata yang valid: {galat:?}",
            info.nama
        );
    }
}
