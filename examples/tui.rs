//! Contoh pemakaian pustaka `ticon` dari program lain — dalam hal ini program
//! yang menggambar sendiri panelnya, seperti TUI pada umumnya.
//!
//! Jalankan dengan:
//!
//! ```text
//! cargo run --example tui
//! ```
//!
//! Empat hal yang sengaja ditunjukkan di sini:
//!
//! * **Tanpa ANSI sama sekali.** `Icon` memberi karakter dan *nama* warna
//!   (`cyan`, `dim`, ...), jadi kita yang memilih gaya. Program TUI sungguhan
//!   menulis ke sel, bukan ke stdout.
//! * **Dua jalur ikon.** Dengan Nerd Font, `icon.ch` berisi glyph asli - butuh
//!   font yang sudah di-patch, dan lebarnya bisa satu atau dua sel. Tanpa Nerd
//!   Font, pack `shape` atau `sempit` lewat `Muat::bentuk_untuk`: nol instalasi
//!   dan tidak ada kotak, karena kalau bentuknya tidak ada hasilnya `None`.
//!   Pack `sempit` menambah jaminan, karakternya cuma `N`/`Na`.
//! * **Lebar glyph harus diputuskan sendiri.** `unicode-width` menghitung
//!   glyph Nerd Font satu sel, sementara banyak font merender dua. TUI harus
//!   memaksa lebarnya agar kolom tidak bergeser; lihat konstanta `SEL_PER_GLYPH`.
//! * **Aturan bisa dijelaskan.** `explain_file` memberi alasan beserta kandidat
//!   yang kalah prioritas - berguna untuk tooltip atau baris status.
//!
//! Paketnya tidak bisa menebak mana yang benar untuk pengguna Anda: program
//! tidak tahu font apa yang terpasang di terminal. Jadi pilihannya harus jadi
//! *setelan* di TUI Anda, bukan tebakan otomatis.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ticon::glyph::Glyphs;
use ticon::mapping::Rules;

/// Berapa sel yang dipakai satu glyph. Dua nilai yang sering dipakai: `2` untuk
/// terminal modern (glyph Nerd Font biasanya dirender lebar) dan `1` kalau
/// font-mu sempit. Kalau kolom meleset, ubah angka ini — itu keputusan TUI,
/// bukan keputusan `ticon`.
const SEL_PER_GLYPH: usize = 2;

/// Peta nama warna `ticon` ke gaya milik program ini sendiri.
#[derive(Default)]
struct Tema {
    /// Warna ticon -> sel gaya yang ditampilkan.
    gaya: BTreeMap<&'static str, &'static str>,
}

impl Tema {
    fn baru() -> Self {
        Self {
            gaya: [
                ("red", "merah"),
                ("green", "hijau"),
                ("yellow", "kuning"),
                ("blue", "biru"),
                ("magenta", "ungu"),
                ("cyan", "sian"),
                ("white", "putih"),
                ("dim", "redup"),
            ]
            .into_iter()
            .collect(),
        }
    }

    fn gaya(&self, warna: &str) -> &str {
        self.gaya.get(warna).copied().unwrap_or("netral")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Muat sekali di awal, pakai berulang. Tidak ada I/O setelah ini:
    //    icons.toml dan assets/glyphs.toml sudah tertanam di biner.
    //
    //    Dua baris di bawah adalah seluruh biaya memakai `ticon` sebagai pustaka:
    //    nol dependensi tambahan, nol proses luar, nol pembacaan berkas saat
    //    berjalan.
    let rules = Rules::load()?;
    let glyphs = Glyphs::bundled();
    let tema = Tema::baru();

    // Jalur bebas-Nerd-Font. Ganti "sempit" dengan "shape" kalau kolom Anda boleh
    // dua sel.
    //
    // Kegagalan memuat pack dibiarkan muncul: pack itu punya salah isi, dan
    // diam-diam kembali ke glyph Nerd Font akan menyembunyikan masalah yang
    // justru ingin dihindari pengguna.
    let bebas_font = ticon::Muat::bawaan()?.dengan_pack("sempit")?;

    // 2. Daftarkan entri seperti seperti TUI: nama, jenis, dan berkasnya.
    let entri: Vec<(PathBuf, bool)> = [
        (PathBuf::from("src/main.rs"), false),
        (PathBuf::from("src/cli.rs"), false),
        (PathBuf::from("app.test.ts"), false),
        (PathBuf::from("README.md"), false),
        (PathBuf::from("Dockerfile"), false),
        (PathBuf::from("assets/logo.png"), false),
        (PathBuf::from(".env"), false),
        (PathBuf::from("latest.py"), false),
        (PathBuf::from("src"), true),
        (PathBuf::from("node_modules"), true),
        (PathBuf::from("belum-dikenal.qqq"), false),
    ]
    .into_iter()
    .collect();

    // 3. Lebar kolom: nama terpanjang, minimum 20, dibulatkan biar rata.
    let lebar_nama = entri
        .iter()
        .map(|(path, _)| path.display().to_string().chars().count())
        .max()
        .unwrap_or(20)
        .max(20)
        .next_multiple_of(4);

    println!("+{}+", "-".repeat(lebar_nama + 30));
    println!(
        "| {:<sel$} {:<sel$} {:<nama$} {:<warna$} |",
        "glif",
        "bebas",
        "nama",
        "warna",
        sel = SEL_PER_GLYPH,
        nama = lebar_nama,
        warna = 6
    );
    println!("+{}+", "-".repeat(lebar_nama + 30));

    for (path, is_dir) in &entri {
        let nama = path.display().to_string();
        let kunci = if *is_dir {
            nama.trim_end_matches('/')
        } else {
            nama.as_str()
        };
        let icon = if *is_dir {
            rules.icon_for_dir(&glyphs, kunci)
        } else {
            rules.icon_for(&glyphs, kunci)
        };

        // Glyph yang tidak ada di tabel sengaja ditampilkan sebagai tanda
        // tanya — lebih jujur daripada diam-diam memakai karakter pengganti.
        let glif = icon
            .ch
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string());

        // Bentuk bebas-font. `bentuk_untuk()` hanya mengembalikan bentuk pack;
        // kalau tidak ada, hasilnya `None` dan kita TIDAK jatuh ke `ch` — itu
        // justru glyph yang tidak bisa digambar. TUI bebas-font lebih baik
        // menampilkan tanpa ikon daripada menampilkan kotak diam-diam.
        let bebas = bebas_font
            .bentuk_untuk(kunci, *is_dir)
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());

        println!(
            "| {:<sel$} {:<sel$} {:<nama$} {:<warna$} |",
            glif,
            bebas,
            nama,
            tema.gaya(&icon.color),
            sel = SEL_PER_GLYPH,
            nama = lebar_nama,
            warna = 6
        );
    }
    println!("+{}+", "-".repeat(lebar_nama + 30));

    // 4. Kalau pengguna bertanya "kenapa file ini dapat ikon itu?", jawabannya
    //    sudah ada — tidak perlu menebak.
    println!();
    for nama in ["app.test.ts", "latest.py", "zzz.qqq"] {
        let penjelasan = rules.explain_file(nama);
        match penjelasan.winner() {
            Some(win) => println!(
                "{nama:<14} -> {} lewat {:?}",
                win.matched_by.label(),
                win.pattern
            ),
            None => println!("{nama:<14} -> tidak ada aturan, pakai kategori bawaan"),
        }
        for kalah in penjelasan.kandidat.iter().skip(1) {
            println!(
                "{:<14}    (kalah: {} {:?})",
                "",
                kalah.matched_by.label(),
                kalah.pattern
            );
        }
    }

    Ok(())
}
