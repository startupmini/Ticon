# Changelog

Semua perubahan yang berarti bagi pengguna dicatat di sini. Formatnya
mengikuti [Keep a Changelog](https://keepachangelog.com/), versinya
[SemVer](https://semver.org/lang/id/).

## 0.2.0

### Ditambahkan

- **Pustaka `ticon`.** `Rules::icon_for` / `icon_for_dir` mengembalikan
  `Icon { ch, color, matched_by }` — karakter, **nama** warna, dan asal aturan.
  Tidak ada string ANSI di dalamnya, jadi pemanggil TUI yang memilih cara
  mewarnainya sendiri.
- **`Rules::explain_file` / `explain_dir`** mengembalikan semua aturan yang cocok
  berurutan prioritas: yang menang dan yang kalah. Ini yang menjawab "kenapa
  `latest.py` bukan ikon test?".
- **`MatchedBy`** — enum asal aturan (`Name`, `Suffix`, `Prefix`, `Extension`,
  `WellKnownFolder`, `Fallback`), sekaligus urutan prioritasnya.
- **`ikon --explain <nama>`** menampilkan penjelasan itu di terminal.
- **`ikon --export`** mengeluarkan seluruh aturan sebagai TSV ke stdout, untuk
  `awk`, skrip shell, dan tool non-Rust.
- `#![warn(missing_docs)]` di pustaka: setiap item publik wajib terdokumentasi,
  dan itu sekarang dijaga `-D warnings` di CI.

### Berubah

- Nama paket menjadi **`ticon`** (nama `ikon` sudah dipakai crate lain di
  crates.io). **Perintahnya tetap `ikon`** — tidak ada yang rusak bagi pemakai
  CLI.
- `Resolved` mendapat field `matched_by`.

### Catatan

- Rilis `0.1.0` dan `0.1.1` tetap bisa dipasang lewat tag-nya; keduanya memuat
  perintah `ikon` dengan perilaku yang sama.

## 0.1.1

- Biner dibangun dari `main` (0.1.0 tertinggal beberapa commit).
- macOS masuk matrix CI.

## 0.1.0

- Rilis pertama: mesin ikon berbasis data, `ikon --list/--gallery/--audit`,
  sanitasi output, dan gerbang audit di build, tes, serta perintah.
