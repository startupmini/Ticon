//! Kontrak yang hanya terlihat dari luar biner: sanitasi nama berkas, penolakan
//! path untuk mode tabel, dan exit code. Tanpa dependensi tambahan — biner
//! dijalankan langsung lewat `CARGO_BIN_EXE_ticon`.
//!
//! Karakter ESC sengaja tidak diuji di sini: Win32 tidak mengizinkan karakter
//! kendali (< 0x20) pada nama berkas, jadi kasus itu ditutup tes unit
//! `render_entry` di `src/lib.rs`. Yang bisa dibuat di semua platform adalah
//! pengendali arah bidi (U+202E), dan itu justru vektor spoofing yang paling
//! sering dipakai di dunia nyata ("invoice\u{202e}fdp.exe").

use std::fs;
use std::process::{Command, Output};

fn ikon(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ticon"))
        .args(args)
        .output()
        .expect("biner ikon harus bisa dijalankan")
}

/// Setup fixture: pack di direktori sementara, lalu `ticon --icons-list-packs`
/// harus menampilkannya apa adanya. Ini yang menangkap bug 0.4.3, di mana
/// `render::sanitize()` mengubah pemisah tab jadi teks `\u{9}` sehingga kolomnya
/// tidak bisa di-cut.
#[test]
fn daftar_pack_menghasilkan_kolom_tab_yang_bisa_dipotong() {
    let dir = std::env::temp_dir().join(format!("ticon-pack-uji-{}", std::process::id()));
    let pack = dir.join("uji").join("pack.json");
    fs::create_dir_all(pack.parent().expect("ada induknya")).expect("direktori dibuat");
    // Delimiter `##` dipakai karena isi JSON memuat `"#` (dari `"kode":"#"`),
    // yang akan menutup raw string `r#"..."#` lebih dulu.
    fs::write(
        &pack,
        r##"{"schema":"ticon-pack/1","pack":{"name":"uji","version":"2.1.0","description":"pack uji","author":"t"},"shapes":{"kode":"#"}}"##,
    )
    .expect("pack ditulis");

    let keluar = Command::new(env!("CARGO_BIN_EXE_ticon"))
        .args(["--icons-list-packs"])
        .env("TICON_PACKS", &dir)
        .output()
        .expect("biner ikon harus bisa dijalankan");
    let _ = fs::remove_dir_all(&dir);

    assert!(
        keluar.status.success(),
        "`--icons-list-packs` gagal: {}",
        String::from_utf8_lossy(&keluar.stderr)
    );
    let teks = String::from_utf8_lossy(&keluar.stdout);

    // Pemisah tab harus tab sungguhan. Di 0.4.3 barisnya masih memuat `\u{9}`
    // sebagai teks, jadi `cut -f1` tidak bisa dipakai.
    assert!(
        !teks.contains("\\u{"),
        "tab tidak boleh jadi teks escapes:\n{teks}"
    );

    let baris = teks
        .lines()
        .find(|b| b.starts_with("uji"))
        .unwrap_or_else(|| panic!("baris pack `uji` tidak ada di:\n{teks}"));
    // Lima kolom: nama, asal, versi, keterangan, penulis. Pemisahnya tab
    // sungguhan - itulah yang membuat `cut -f3` berguna.
    let kolom: Vec<&str> = baris.split('\t').collect();
    assert_eq!(kolom.len(), 5, "kolom: {kolom:?}");
    assert_eq!(kolom[0].trim(), "uji");
    assert!(kolom[1].ends_with("pack.json"), "asal: {}", kolom[1]);
    assert_eq!(kolom[2], "2.1.0");
    assert_eq!(kolom[3], "pack uji");
    assert_eq!(kolom[4], "(t)");
}

#[test]
fn audit_keluar_nol_saat_pemetaan_bersih() {
    let hasil = ikon(&["--audit"]);
    assert!(
        hasil.status.success(),
        "`--audit` gagal padahal icons.toml bersih: {}",
        String::from_utf8_lossy(&hasil.stderr)
    );
}

#[test]
fn list_ditolak_untuk_path() {
    let hasil = ikon(&["--list", "src"]);
    assert!(!hasil.status.success(), "`--list` seharusnya menolak path");
    let pesan = String::from_utf8_lossy(&hasil.stderr);
    assert!(pesan.contains("--list") && pesan.contains("src"), "{pesan}");
    // Prefix pesan galat harus nama binernya. Ini pernah `ikon:` padahal
    // binernya `ticon`, jadi pengguna melihat sesuatu yang tidak bisa mereka
    // panggil.
    assert!(
        pesan.starts_with("ticon:"),
        "prefix harus `ticon:`, dapat: {pesan}"
    );
}

#[test]
fn peta_netral_tertulis_tangan_dipakai() {
    // Peta kustom dibaca lewat `--icons-map`: ini yang menutup klaim "satu
    // berkas netral bisa dipakai ticon juga", yang di 0.4.0 hanya benar
    // separuh.
    let dir = std::env::temp_dir().join("ikon-e2e-peta");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("folder uji harus bisa dibuat");
    let peta = dir.join("icons.json");
    fs::write(
        &peta,
        r#"{
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
}"#,
    )
    .expect("peta uji harus bisa ditulis");

    let hasil = ikon(&[
        "--icons-map",
        peta.to_str().expect("path harus UTF-8"),
        "--explain",
        "berkas.myp",
    ]);
    let _ = fs::remove_dir_all(&dir);
    assert!(
        hasil.status.success(),
        "--icons-map gagal: {}",
        String::from_utf8_lossy(&hasil.stderr)
    );
    let keluar = String::from_utf8_lossy(&hasil.stdout);
    // `.myp` tidak ada di peta bawaan, jadi kalau menang berarti peta kustom
    // benar-benar dipakai. Yang dicetak `--explain` adalah karakter dan warnanya,
    // bukan nama glyph-nya.
    assert!(
        keluar.contains(".myp"),
        "aturan dari peta kustom harus menang: {keluar}"
    );
    assert!(
        keluar.contains("green"),
        "warna harus ikut dari peta kustom: {keluar}"
    );
}

#[test]
fn audit_temuan_mempunyai_satu_prefix_saja() {
    // `audit()` mengembalikan laporannya sebagai `Err`, dan `run()` menambahkan
    // `ticon: ` sendiri. Kalau laporannya sudah berprefix, yang keluar adalah
    // `ticon: ticon: …`. Kecacatan ini ada sejak 0.4.0 dan tidak pernah
    // tertangkap karena tidak ada tes yang men-trigger temuan audit.
    let dir = std::env::temp_dir().join("ikon-e2e-audit-temuan");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("folder uji harus bisa dibuat");
    let peta = dir.join("icons.json");
    // Dua keluarga berbagi satu warna — persis temuan yang dicari `audit()`.
    fs::write(
        &peta,
        r#"{
  "schema": "ticon-map/2",
  "families": { "kerja": "green", "dokumen": "green" },
  "defaults": {
    "file": { "glyph": "nf-md-file_outline", "family": "kerja", "fallback": "F" },
    "dir": { "glyph": "nf-md-folder_outline", "family": "kerja", "fallback": "D" }
  },
  "rules": [
    { "kind": "ext", "priority": 4, "key": ".aa1", "family": "kerja",
      "glyph": "nf-md-language_rust", "codepoint": 988695, "fallback": "rs" },
    { "kind": "ext", "priority": 4, "key": ".bb2", "family": "dokumen",
      "glyph": "nf-md-language_go", "codepoint": 985043, "fallback": "go" }
  ]
}"#,
    )
    .expect("peta uji harus bisa ditulis");

    let hasil = ikon(&["--icons-map", peta.to_str().expect("path"), "--audit"]);
    let _ = fs::remove_dir_all(&dir);

    assert!(
        !hasil.status.success(),
        "temuan audit harus bikin exit code bukan nol"
    );
    let pesan = String::from_utf8_lossy(&hasil.stderr);
    assert!(pesan.contains("masalah konsistensi"), "{pesan}");
    assert!(
        pesan.starts_with("ticon:") && !pesan.contains("ticon: ticon:"),
        "harus tepat satu prefix `ticon:`, dapat: {pesan}"
    );
}

#[test]
fn peta_netral_dengan_skema_lama_ditolak_dengan_petunjuk() {
    let dir = std::env::temp_dir().join("ikon-e2e-peta-lama");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("folder uji harus bisa dibuat");
    let peta = dir.join("icons.json");
    fs::write(&peta, r#"{"schema":"ticon-map/1"}"#).expect("peta uji ditulis");

    let hasil = ikon(&["--icons-map", peta.to_str().expect("path"), "--audit"]);
    let _ = fs::remove_dir_all(&dir);

    assert!(!hasil.status.success(), "skema lama harus ditolak");
    let pesan = String::from_utf8_lossy(&hasil.stderr);
    assert!(
        pesan.contains("--export=json"),
        "pesan harus mengarahkan ekspor ulang: {pesan}"
    );
}

#[test]
fn nama_dengan_pengendali_bidi_tidak_mentah_di_keluaran() {
    let dir = std::env::temp_dir().join("ikon-e2e-bidi");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("folder uji harus bisa dibuat");
    let nama = "invoice\u{202e}fdp.exe";
    fs::write(dir.join(nama), "").expect("berkas uji harus bisa ditulis");

    let hasil = ikon(&[
        "-1",
        "--icons",
        "always",
        "--color",
        "never",
        "--width",
        "200",
        dir.to_str().expect("path uji harus valid UTF-8"),
    ]);
    let _ = fs::remove_dir_all(&dir);

    assert!(hasil.status.success(), "list gagal: {hasil:?}");
    let keluar = String::from_utf8_lossy(&hasil.stdout);
    assert!(
        !keluar.contains('\u{202e}'),
        "U+202E mentah sampai ke stdout: {keluar}"
    );
    assert!(
        keluar.contains("\\u{202e}"),
        "harus tampil sebagai representasi aman: {keluar}"
    );
}
