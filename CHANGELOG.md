# Changelog

Semua perubahan yang berarti bagi pengguna dicatat di sini. Formatnya
mengikuti [Keep a Changelog](https://keepachangelog.com/), versinya
[SemVer](https://semver.org/lang/id/).

## Belum rilis

### Ditambahkan

- **`--icons shape`**: mode tanpa Nerd Font. Bentuk satu sel per keluarga
  warna (`≡ → ● § ▲ ♦ □ ▪`, dan `▼` untuk folder), dipilih dari karakter yang
  **terbukti** ada di font dasar — `tools/audit-font.py` mengeceknya ke tujuh
  font yang biasa ada di Windows dan keluar dengan error kalau ada yang tidak
  ada. Alasannya: glyph Nerd Font ada di codepoint private-use, jadi pengguna
  tanpa font itu melihat kotak, dan program tidak bisa mendeteksinya dari
  dalam proses.
- **Subset font Nerd Fonts ikut dikemas** (`assets/raster.ttf`, 14,1 KiB dari
  2,2 MB; 55 glyph). Bukan untuk jalur karakter — terminal tetap memakai font
  miliknya sendiri — tapi sebagai input jalur render half-block, supaya
  konsumen tidak perlu mengunduh 2,2 MB.
  `tests/subset-font.rs` menjaga subset itu tetap pas: gagal kalau
  `icons.toml` memakai glyph yang tidak ada di subset, dan juga kalau subsetnya
  membawa glyph yang tidak terpakai.
- **`tools/gen-raster.py`**: membuat subset dan `src/raster_data.rs` dari
  `icons.toml`, dengan verifikasi SHA-256 arsip upstream.

### Diperbaiki

- README mengklaim proyek ini tidak ada di crates.io; sebenarnya sudah terbit,
  jadi cara pasangnya (termasuk `cargo add ticon`) diperbarui.

## 0.4.2

Rilis keamanan: jalur pemuatan peta netral (`--icons-map`, diperkenalkan di
0.4.1) **melewati** gerbang karakter kontrol yang sudah ada untuk `icons.toml`.
Temuan ini berasal dari tinjauan keamanan terhadap kode sendiri, bukan dari
laporan pihak ketiga.

### Diperbaiki

- **Injeksi terminal lewat peta netral.** `SECURITY.md` sudah menyatakan data
  pemetaan ditolak bila memuat karakter kontrol/bidi — tapi pemeriksaan itu hanya
  ada di `Rules::load_from` untuk `icons.toml`. Satu karakter ESC pada `key`,
  `fallback`, atau nama keluarga di `icons.json` akan ikut ke output terminal.
  Sekarang **setiap** string dari peta melewati `cek_aman` sebelum masuk `Rules`,
  lewat satu titik rempit (`wajib_teks`/`opsional_teks`) supaya field baru di
  kemudian hari tidak bisa melewatinya.
- **Pesan galat jadi vektor kedua.** Nama kunci yang tidak dikenal ikut
  disanitasi sebelum dicetak, jadi pesan penolakan tidak bisa menyuntikkan ESC.
- **Peta tanpa batas ukuran.** `--icons-map` membaca berkas sepenuhnya; kini
  berkas di atas 4 MiB ditolak berdasarkan metadata, sebelum isinya dimuat.
- **Kunci aturan kembar.** Dua aturan dengan `kind` + `key` sama dulu memilih
  yang pertama diam-diam; sekarang ditolak, karena "mana yang menang?" tidak
  punya jawaban yang bisa ditebak.

### Ditambahkan

- **`tests/keamanan.rs`** (9 tes) untuk gerbang jalur peta netral. Enam di
  antaranya dibuktikan **gagal pada build rentan** lewat uji diferensial, jadi
  menguji perbaikan dan bukan sekadar ikut hijau.
- BOM di depan peta dilewati, bukan ditolak dengan pesan yang membingungkan.

### Tidak berubah

Kolom TSV, kontrak `ticon-map/2`, dan perilaku bawaan perintah tidak berubah.

## 0.4.1

Rilis ini memperbaiki beberapa hal yang terlewat di 0.4.0 — sebagian di antaranya
saya temukan sendiri setelah scrutinize 0.4.0. Rinciannya di bawah, termasuk
alasan di balik keputusan yang bisa diperdebatkan.

### Berubah

- **Kontrak JSON naik ke `ticon-map/2` dengan nama field netral.** `kind`, `key`,
  `family`, `color`, `width`, `fallback`, `order`, `families`, `defaults`,
  `rules`. `ticon-map/1` (dikirim bersama 0.4.0) memakai nama berbahasa
  Indonesia dan **tidak kompatibel**; pemuatnya menolak berkas itu dengan pesan
  yang mengarahkan ekspor ulang. Yang tetap bahasa Indonesia: dokumentasi dan
  API Rust, sesuai bahasa repo ini. Yang di-English-kan hanya *identifier pada
  wire format*, karena itu yang dibaca pihak ketiga di luar repo.
- **`width_default` turun dari 2 ke 1.** Nilai 2 dulu hanya warisan perilaku
  renderer lama, tidak pernah diukur, dan banyak terminal modern merender glyph
  Nerd Font satu sel. `ticon` sendiri tidak memakainya untuk merender.
- **`width` dan `fallback` hanya ditulis kalau diisi.** Di 0.4.0 setiap aturan
  membawa `"fallback": "?"`, jadi field itu tidak pernah berarti. Lebih baik
  tidak ada daripada isinya menebak. Dua kategori bawaan (`file`, `folder`) kini
  punya `fallback` sungguhan.
- **`order` tidak lagi memuat `default`.** `default` adalah hasil akhir, bukan
  tahap pencocokan; menyatakannya di `order` hanya laporan implementasi
  dan membuat kode perlu kasus khusus. Nilai `kind` pada tiap aturan juga memakai
 token yang sama.
- **`--export` bisa memakai bentuk dengan spasi** (`--export json`) dan ada
  alias `--format json`. Sebelumnya hanya `--export=json` yang dikenali;
  `--export json` diam-diam menghasilkan TSV.

### Ditambahkan

- **`ticon --icons-map <berkas>`**: peta netral bisa dipakai `ticon` itu sendiri,
  jadi satu berkas benar-benar bisa dibuat siapa saja — termasuk ditulis tangan —
  lalu dipakai oleh `ticon` maupun konsumen lain. Nilai yang tidak dikenal
  **ditolak** (bukan diabaikan diam-diam), dan glyph harus sudah ada di tabel
  bawaan; peta kustom belum bisa memunculkan glyph baru lewat `ticon`.
- **`schema/ticon-map-2.json`**: JSON Schema (draft 2020-12) untuk kontrak ini,
  dan ekspor menyertakan `$schema` supaya editor bisa memvalidasinya.
- **`tests/ekspor.rs`**: memeriksa bahwa JSON benar-benar bisa diurai, lengkap,
  konsisten dengan TSV, semua glyph punya codepoint yang cocok, dan **ekspor
  bolak-balik tidak mengubah isi**. Ini menutup kelas bug yang tidak terlihat
  dari korpus konformansi (mis. `codepoint` tercetak sebagai karakter).
- **Parser JSON sendiri** (`src/json.rs`, std-only) dengan batas kedalaman,
  penolakan input rusak, dan dukungan escape Unicode termasuk pasangan
  surrogate.
- **Job `publish` di `release.yml`**: crates.io kini terbit lewat workflow
  (dengan persetujuan manual lewat `environment`, dan cek versi tag lebih
  dulu), bukan `cargo publish` manual yang mudah terlupa.
- **`ekspor_json` dan `ekspor_tsv` jadi API publik**, jadi skrip Rust bisa
  mengambil peta tanpa memproses biner.
- **Korpus konformansi diperluas** dengan nama ber-spasi, unicode, emoji,
  sangat panjang, dan huruf besar semua.

### Diketahui masih belum

- Glyph baru di peta kustom belum bisa dipakai `ticon` (hanya lewat konsumen lain
  yang punya tabel glyph sendiri). Pesan penolakannya menyebut ini.
- `examples/ts/` adalah referensi, bukan starter kit: butuh Node ≥ 22.6 untuk
  `--experimental-strip-types`, dan `import "./ikon.ts"` perlu
  `allowImportingTsExtensions` di proyek tsc biasa.

## 0.4.0

### Ditambahkan

- **`ticon --export=json`**: peta ikon sebagai satu berkas JSON berversi
  (`ticon-map/1`) — kontrak netral untuk konsumen non-Rust. Yang membuatnya
  tidak terikat: urutan resolver ikut keluar sebagai angka `priority`, warna
  bawaan (`warna`) tinggal satu field yang boleh diabaikan (yang dipakai
  memetakan `keluarga` ke gaya sendiri), dan `lebar` serta `fallback` ikut
  dibawa supaya aplikasi tanpa Nerd Font tetap bisa merender sesuatu.
- **`Prioritas` jadi API publik** dengan angka 1-5. `resolve_*`, `explain_*`, dan
  ekspor JSON membaca enum yang sama, jadi urutan resolusi tidak ditulis dua
  kali di dua bahasa lagi.
- **Kolom opsional `lebar` dan `fallback` per kategori** di `icons.toml`.
- **`examples/ts/`**: resolver referensi TypeScript yang membaca
  `icons.json`, memetakan warna ke gaya sendiri, dan memutuskan lebar selnya
  sendiri.
- **Korpus konformasi** (`tests/konformasi/`): 51 entri nama berkas dan folder
  beserta hasil yang diharapkan. Dipakai tes Rust **dan** `examples/ts/cek.ts`,
  jadi resolver di bahasa lain bisa membuktikan dirinya sama tanpa bergantung
  pada `ticon` saat runtime. Job CI baru menjalankannya di Linux.

### Berubah

- `ticon --export` tanpa nilai tetap TSV seperti sebelumnya; bentuk JSON diambil
  lewat `--export=json` (hanya nilai inline, supaya `--export` tidak ikut
  menelan nama path yang menyusul).

## 0.3.1

### Ditambahkan

- `examples/tui.rs`: contoh pemakaian pustaka dari program yang menggambar
  panelnya sendiri. Menunjukkan tiga hal yang jadi keputusan konsumen, bukan
  keputusan `ticon`: memetakan nama warna ke gaya sendiri, lebar sel glyph,
  dan menampilkan alasan sebuah ikon dipilih.
  ```bash
  cargo run --example tui
  ```

## 0.3.0

### Berubah

- **Perintahnya sekarang `ticon`, bukan `ikon`.** Sekaligus menyelaraskan
  perintah dengan nama paket dan pustakanya. Perilaku, opsi, dan keluarannya
  tidak berubah — hanya nama yang dipanggil.
  ```bash
  # sebelum
  ikon --list
  # sekarang
  ticon --list
  ```
- Variabel lingkungan `IKON_ICONS` dan `IKON_COLOR` diganti jadi
  `TICON_ICONS` dan `TICON_COLOR`. **Nama lama masih dibaca sebagai alias**,
  jadi konfigurasi yang sudah ada tidak langsung mati.

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
