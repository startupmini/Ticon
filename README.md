# ticon

Ikon minimalis dan flat untuk terminal.

`ticon` menampilkan nama berkas bersama satu glyph yang menunjukkan jenisnya,
diwarnai menurut kategori. Setiap ikon berasal dari satu keluarga glyph yang
sama, dan paletnya cuma delapan warna — itulah yang membuatnya terasa tenang,
bukan seperti daftar logo.

```
󰅩 src   󰙨 app.test.ts  󰟓 main.go  󰗚 README.md   󰞹 backup.tar.gz  󰯅 .env
```

## Dua aturan yang ditegakkan mesin

Desain yang cuma ditulis di dokumen akan luntur. Dua prinsip di bawah ini punya
penjaganya sendiri:

1. **Hanya glyph `nf-md-` (Material Design Icons).** Semuanya digambar pada
   grid 24px yang sama dengan ketebalan stroke seragam. Satu glyph dari
   keluarga lain sudah cukup untuk merusak kesan flat — jadi `build.rs`
   memeriksanya, dan salah keluarga berarti gagal compile.
2. **Maksimum delapan warna, satu warna per keluarga.** Seluruh bahasa
   pemrograman berbagi satu warna; bedanya dibawa oleh bentuk ikon. Setiap
   aturan menyatakan keluarganya di tabel `[families]` — warnanya diturunkan
   dari situ, jadi "siapa berbagi warna dengan siapa" terbaca dari satu
   tabel, bukan ditebak dari 64 baris. Paletnya diambil dari 16 warna ANSI
   terminal, jadi otomatis selaras (dan ikut berubah) saat kamu ganti theme
   terminal. Test memegang empat sekaligus: warna kesembilan = gagal, dua
   keluarga berebut satu warna = gagal, dua tetangga beda keluarga yang
   sewarna = gagal, dan `dim` melebihi 10 aturan = gagal.

## Dokumentasi

* [`docs/konfigurasi.md`](docs/konfigurasi.md) — struktur `icons.toml` seluruhnya:
  palet, keluarga warna, kategori, urutan resolusi, dan cara menambah aturan.
* [`docs/desain.md`](docs/desain.md) — mengapa desainnya begini: delapan warna,
  satu keluarga satu warna, batas `dim`, serta aturan ketetanggaan.

## Pasang

Proyek ini dikirim lewat GitHub, bukan crates.io — nama `ticon` sudah dipakai
crate lain di sana, dan aplikasi ini memang tidak dimaksudkan menjadi pustaka
yang diimpor.

```bash
# dari tag rilis
cargo install --git https://github.com/startupmini/Ticon --tag v0.3.0

# atau langsung dari branch utama
cargo install --git https://github.com/startupmini/Ticon
```

Trial tanpa memasang sama sekali:

```bash
cargo run --quiet -- <args>
```

Kebutuhan minimum: Rust 1.85 — angka ini berasal dari dependensi (bukan dari
kode `ticon` sendiri) dan dijaga job `msrv` di CI. Diuji di Linux dan Windows;
lebar terminal di macOS/Linux diambil lewat `ioctl`.

## Pakai

```
ikon                      # daftar direktori saat ini
ikon src/                 # beberapa path sekaligus juga bisa
ikon -a --sort size       # termasuk berkas tersembunyi, urut ukuran
ikon -1                   # satu entri per baris
ticon --list               # cetak seluruh tabel pemetaan, untuk ditinjau
ticon --gallery            # cetak contoh ikon dari tiap aturan
ticon --audit              # periksa konsistensi icons.toml
ticon --explain main.go    # kenapa berkas ini dapat ikon itu
ticon --export             # semua aturan sebagai tabel TSV
```

| Opsi | Arti |
|---|---|
| `-a`, `--all` | tampilkan berkas tersembunyi |
| `-1` | satu entri per baris |
| `--icons <mode>` | `auto`, `always`, `never` (bawaan `auto`) |
| `--color <mode>` | `auto`, `always`, `never` (bawaan `auto`) |
| `--sort <kunci>` | `name`, `ext`, `size`, `time` (bawaan `name`) |
| `--width <kolom>` | paksa lebar tata letak |
| `--list` | cetak tabel pemetaan (tanpa path) |
| `--gallery` | cetak contoh ikon dari tiap aturan (tanpa path) |
| `--audit` | periksa konsistensi pemetaan |
| `--explain <nama>` | tampilkan aturan yang menang **dan** yang kalah prioritas |
| `--export` | cetak semua aturan sebagai TSV ke stdout |

Alias yang didukung: `--colour` (sama dengan `--color`), `--no-color` (sama
dengan `--color never`), `--sort extension` / `--sort mtime`, dan `--` untuk
menghentikan parsing opsi.

## Pakai sebagai pustaka

Paketnya bernama **`ticon`**; perintahnya tetap `ticon`. Pustakanya untuk
aplikasi TUI, previewer, atau apa pun yang butuh tauhu  ikon sebuah nama
tanpa memanggil proses luar:

```rust
let rules = ticon::mapping::Rules::load()?;
let glyphs = ticon::glyph::Glyphs::bundled();

let icon = rules.icon_for(&glyphs, "main.rs");
println!("{:?} {} {:?}", icon.ch, icon.color, icon.matched_by);
// Some('󱘗') "cyan" Suffix
```

Yang dikembalikan **bukan string ANSI**: `Icon` berisi karakter glyph, **nama**
warna, dan asal pencocokannya, jadi pemanggil yang memilih cara mewarnainya.
Kalau butuh alasannya secara rinci:

```rust
let penjelasan = rules.explain_file("app.test.ts");
println!("{:?}", penjelasan.winner()); // Some(Kandidat { matched_by: Suffix, pattern: ".test.ts", .. })
```

Untuk tool non-Rust, `ticon --export` mengeluarkan seluruh aturan sebagai TSV
yang bisa langsung dibaca `awk` atau skrip shell.

| Variabel lingkungan | Arti |
|---|---|
| `NO_COLOR` | matikan warna kalau diisi (nilai apa pun) |
| `CLICOLOR_FORCE` | paksa ikon & warna walau bukan terminal (selain `0`) |
| `TICON_ICONS` | nilai bawaan untuk `--icons` |
| `TICON_COLOR` | nilai bawaan untuk `--color` |
| `COLUMNS` | lebar kolom kalau terminal tidak bisa dideteksi |

Ikon memerlukan font yang sudah di-patch Nerd Fonts.

Kalau terminalmu tidak terdeteksi sebagai terminal (MinTTY di Git Bash paling
sering), ikon dan warna dimatikan otomatis supaya output tetap bersih saat
disalurkan ke program lain. Untuk memaksanya:

```
export TICON_ICONS=always
export TICON_COLOR=always
```

## Cara kerjanya

```
nama berkas ──► Rules::resolve_*──► nama glyph + nama warna
                                       │
                        Glyphs ────────┴──► karakter + kode ANSI ──► render
```

Yang penting dari bagan itu: resolver tidak pernah tahu karakter apa yang
dipakai, dan renderer tidak pernah tahu aturan pencocokannya. Karena itu
`icons.toml` bisa diubah tanpa menyentuh kode sama sekali.

```
icons.toml            sumber kebenaran tunggal: bentuk + keluarga warna + cakupan
assets/glyphs.toml    nama glyph → codepoint (hasil generate, ikut di-commit)
build.rs              lint saat build: keluarga glyph & keberadaan glyph
src/mapping.rs        pemuatan, indeks, resolver, dan audit
src/render.rs         warna ANSI + tata letak grid
src/terminal.rs       deteksi lebar terminal, tanpa dependensi
tools/gen-glyphs.py   memangkas tabel glyph upstream
docs/                 dokumentasi konfigurasi & desain
```

Urutan pencocokan, dari yang paling spesifik: nama folder well-known → nama
berkas persis → akhiran (`main_test.go`) → awalan (`README`, `LICENSE`) →
ekstensi terpanjang (`.tar.gz` menang atas `.gz`) → bawaan.

Untuk menambah jenis berkas baru, cukup tambahkan string ke daftar di
`icons.toml`. Kamu tidak perlu menambah ikon baru.

## Asal data

* Codepoint glyph diambil dari data resmi
  [Nerd Fonts](https://github.com/ryanoasis/nerd-fonts) v3.4.0 (MIT) lewat
  `tools/gen-glyphs.py`. Codepoint adalah fakta, dan hanya glyph yang dipakai
  yang masuk repo — 55 dari 10.764.
* Kode di proyek ini ditulis dari nol. Konsep "tampilkan ikon di terminal"
  bukan hal baru, dan ada banyak proyek lain di sana, tapi tidak ada baris kode
  maupun tabel pemetaan yang disalin dari proyek mana pun.

## Lisensi

MIT — berkas [LICENSE](LICENSE). Data glyph berasal dari Nerd Fonts (MIT).

## Pengembangan

```bash
cargo test                              # unit + integrasi + audit pemetaan
cargo run -- --audit
python tools/gen-glyphs.py --refresh    # regenerasi tabel glyph
```

Testnya memuat audit yang sama dengan `--audit`, jadi pemetaan yang tidak
konsisten tidak akan pernah sampai ke rilis.

Di Windows, toolchain `stable-x86_64-pc-windows-gnu` berjalan tanpa Visual
Studio Build Tools. Ketergantungan sengaja dijaga bebas dari crate yang memuat
`windows-sys`, karena itu memaksa `dlltool` untuk membuat import library dan
justru gagal di toolchain tersebut.
