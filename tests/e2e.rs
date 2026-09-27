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
