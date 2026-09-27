//! Contoh pemakaian pustaka `ticon` dari program lain — dalam hal ini program
//! yang menggambar sendiri panelnya, seperti TUI pada umumnya.
//!
//! Jalankan dengan:
//!
//! ```text
//! cargo run --example tui
//! ```
//!
//! Tiga hal yang sengaja ditunjukkan di sini:
//!
//! 1. **Tanpa ANSI sama sekali.** `Icon` memberi karakter dan *nama* warna
//!    (`cyan`, `dim`, ...), jadi kita yang memilih gaya. Program TUI sungguhan
//!    menulis ke sel, bukan ke stdout.
//! 2. **Lebar glyph harus diputuskan sendiri.** `unicode-width` menghitung
//!    glyph Nerd Font satu sel, sementara banyak font merender dua. TUI harus
//!    memaksa lebarnya agar kolom tidak bergeser; lihat konstanta `SEL_PER_GLYPH`.
//! 3. **Aturan bisa dijelaskan.** `explain_file` memberi alasan beserta kandidat
//!    yang kalah prioritas — berguna untuk tooltip atau baris status.

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
    let rules = Rules::load()?;
    let glyphs = Glyphs::bundled();
    let tema = Tema::baru();

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

    println!("+{}+", "-".repeat(lebar_nama + 22));
    println!(
        "| {:<sel$} {:<nama$} {:<warna$} |",
        "glyf",
        "nama",
        "warna",
        sel = SEL_PER_GLYPH,
        nama = lebar_nama,
        warna = 6
    );
    println!("+{}+", "-".repeat(lebar_nama + 22));

    for (path, is_dir) in &entri {
        let nama = path.display().to_string();
        let icon = if *is_dir {
            rules.icon_for_dir(&glyphs, nama.trim_end_matches('/'))
        } else {
            rules.icon_for(&glyphs, &nama)
        };

        // Glyph yang tidak ada di tabel sengaja ditampilkan sebagai tanda
        // tanya — lebih jujur daripada diam-diam memakai karakter pengganti.
        let glif = icon
            .ch
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string());

        println!(
            "| {:<sel$} {:<nama$} {:<warna$} |",
            glif,
            nama,
            tema.gaya(&icon.color),
            sel = SEL_PER_GLYPH,
            nama = lebar_nama,
            warna = 6
        );
    }
    println!("+{}+", "-".repeat(lebar_nama + 22));

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
