# Konfigurasi

Seluruh perilaku `ticon` — bentuk ikon, warnanya, dan berkas mana yang cocok —
hidup di satu file: [`icons.toml`](../icons.toml). Kode tidak memuat tabel
kembar, jadi mustahil menambah ikon tanpa sekaligus memutuskan warnanya.

```text
icons.toml
├── [palette]       8 warna terminal + kode ANSI-nya
├── [families]      keluarga → warna (satu keluarga satu warna)
├── [dirs]          folder well-known: nama → glyph + keluarga
└── [categories.*]  kategori berkas: cakupan + glyph + keluarga
    └── [categories.<nama>.by_ext]   bentuk khusus per ekstensi (opsional)
```

## [palette]

| nama | ANSI | peran |
|---|---|---|
| `red` | 31 | keluarga `aman` |
| `green` | 32 | keluarga `kerja` |
| `yellow` | 33 | keluarga `data` |
| `blue` | 34 | keluarga `dokumen` |
| `magenta` | 35 | keluarga `media` |
| `cyan` | 36 | keluarga `kode` |
| `white` | 37 | keluarga `netral` |
| `dim` | 90 | keluarga `redup` |

Nilai adalah kode ANSI SGR, bukan warna truecolor. Konsekuensinya: palet
otomatis mengikuti theme terminal kamu — ganti theme, ikon ikut berubah.
`NO_COLOR`, `CLICOLOR_FORCE`, dan `TICON_COLOR` tetap berlaku. Nilainya harus
angka: `load()` menolak apa pun yang bukan SGR numerik, karena kode itu masuk
ke `\x1b[{kode}m` apa adanya.

Palet dibatasi 8 entri. Test `palet_tidak_lebih_dari_delapan_warna` gagal
begitu warna kesembilan ditambahkan, supaya keputusan itu selalu diambil sadar.

## [families]

Kelompok yang berbagi warna dinyatakan eksplisit, bukan ditebak dari 64 baris:

```toml
[families]
kode = "cyan"    # bahasa, kontainer, infra kode, folder sumber
# ...

[categories.code]
glyph  = "nf-md-code_braces"
family = "kode"        # warnanya cyan — diturunkan, bukan ditulis
```

| keluarga | warna | aturan | isi |
|---|---|---|---|
| `kode` | cyan | 8 | bahasa, container, cloud, folder sumber |
| `kerja` | green | 6 | shell, test, task, folder uji |
| `data` | yellow | 9 | data, markup, arsip, lockfile, paket, database |
| `media` | magenta | 10 | gaya, gambar, audio, video, slide, model, font |
| `dokumen` | blue | 7 | folder bawaan, note, readme, pdf, word, docs |
| `aman` | red | 7 | secret, vcs, key, cert, folder `.git` |
| `netral` | white | 8 | teks polos, config, log, biner |
| `redup` | dim | 9 | berkas tak dikenal + artefak hasil generate |

Yang ditegakkan (di `load()`, `cargo test`, dan `--audit`):

* setiap kategori dan folder **wajib** menyebut `family` yang ada di tabel —
  selain itu muat file gagal dengan pesan yang menunjuk pelakunya;
* **tidak ada karakter kontrol/bidi** di string mana pun (`names`, `prefix`,
  `ext`, kunci folder, ...) — `load()` menolaknya, sebab satu karakter ESC di
  nama sudah cukup untuk menyuntikkan sekuens ke terminal yang menjalankan
  `--list`; nama berkas dari sistem berkas disanitasi saat dicetak;
* **satu keluarga satu warna, satu warna satu keluarga** — dua keluarga tidak
  boleh berebut satu warna;
* tidak ada warna palet yang menganggur dan tidak ada keluarga tanpa anggota;
* dua aturan yang **bersebelahan di file** hanya boleh sewarna kalau satu
  keluarga (lihat [desain](desain.md#ketetanggaan));
* dua kategori tidak boleh **berebut `prefix`/`suffix` yang sama** — pemenangnya
  dipilih diam-diam oleh urutan alfabet, jadi audit yang menagih;
* kunci `[dirs]` yang hanya beda kapitalisasi (`Src` vs `src`) adalah kunci
  yang sama — `load()` menolak tabrakan seperti itu;
* pemakaian `dim` maksimal `MAX_DIM_RULES` = 10 aturan.

Ringkasan keluarga ini ikut tercetak oleh `ticon --list`, dan `ticon --gallery`
menampilkan satu contoh ikon per aturan — keduanya cara cepat meninjau dampak
perubahan warna tanpa membaca file mentah.

## [dirs]

```toml
[dirs]
src    = { glyph = "nf-md-code_braces",      family = "kode" }
".git" = { glyph = "nf-md-source_branch",    family = "aman" }
```

* nama dicocokkan tanpa peduli huruf besar/kecil (semua diturunkan huruf
  kecil saat dimuat), jadi `SRC` dan `src` sama;
* kunci berawalan titik ditulis dalam tanda kutip;
* yang tidak cocok jatuh ke kategori bawaan `folder`.

## [categories.<nama>]

| field | wajib | arti |
|---|---|---|
| `glyph` | ✓ | glyph Nerd Fonts, wajib dari keluarga `nf-md-`/`nf-oct-` |
| `family` | ✓ | keluarga dari `[families]` — di sinilah warnanya dipilih |
| `ext` | – | daftar ekstensi, huruf kecil, diawali titik (`.tar.gz` sah) |
| `names` | – | nama berkas persis (`package.json`, `.gitignore`) |
| `prefix` | – | awalan nama (`readme`, `.env.`) |
| `suffix` | – | akhiran nama (`_test.go`, `.spec.ts`) |
| `by_ext` | – | glyph berbeda per ekstensi, **warna tetap milik kategori** |
| `lebar` | – | berapa sel yang dipakai glyph kategori ini (petunjuk, bukan aturan) |
| `fallback` | – | teks pendek kalau glyph tidak bisa ditampilkan |

`by_ext` adalah cara memvariasikan bentuk tanpa menambah warna: seluruh bahasa
pemrograman tetap satu warna, bedanya dibawa glyph. Kunci `by_ext` wajib ada
di daftar `ext` — audit menolak kunci yang menggantung.

`lebar` dan `fallback` menentukan lebar kolom dan apa yang ditampilkan saat font
tidak punya glyph-nya — berguna bagi konsumen non-terminal (TUI, web, LSP). Keduanya
opsional: tanpa `lebar` diasumsikan 2 sel, tanpa `fallback` dipakai `?`. Sampai
sini keduanya baru ikut diekspor; perintah `ticon` sendiri belum memakainya.

Catatan pencocokan: `ext` selalu huruf kecil (dicek audit); `names`, `prefix`,
dan `suffix` tidak peduli huruf besar/kecil.

## Urutan resolusi

Dari yang paling spesifik:

1. nama folder well-known (`[dirs]`)
2. nama file persis (`names`)
3. akhiran nama (`suffix`)
4. awalan nama (`prefix`)
5. ekstensi terpanjang
6. bawaan: `folder` untuk folder, `file` untuk file

Nomornya berasal dari enum `Prioritas` (1-5) yang sama dengan angka `priority`
di `icons.json`, jadi urutan ini tidak ditulis ulang di implementasi lain.

Dua pemenang yang sering ditanyakan:

* `app.test.ts` → **berkas uji**, bukan TypeScript — akhiran didahulukan
  supaya `.test.ts` menang atas `.ts`;
* `arsip.tar.gz` → **arsip**, bukan `.gz` biasa — ekstensi terpanjang menang.

## Menambah aturan baru

Contoh: kategori `cad` yang berbagi glyph dengan `model` (boleh — glyph boleh
dipakai banyak kategori):

```toml
[categories.cad]
glyph  = "nf-md-cube_outline"
family = "media"
ext    = [".dwg", ".dxf"]
```

Folder baru:

```toml
themes = { glyph = "nf-md-palette_outline", family = "media" }
```

Checklist:

1. Pakai keluarga yang sudah ada. Delapan sudah penuh — test menolak warna
   kesembilan.
2. Periksa tetangganya di file: beda keluarga tidak boleh sewarna (atau
   pindahkan ke keluarga yang sama dengan tetangganya).
3. Kalau memakai `redup`, ingat batas 10 aturan.
4. Glyph yang belum ada di `assets/glyphs.toml` harus diregenerasi dulu:
   `python tools/gen-glyphs.py` (butuh jaringan). Kalau lupa, `build.rs`
   menjelaskan persis glyph mana yang hilang.
5. Jalankan `cargo test` dan `ticon --audit` — keduanya memeriksa hal yang sama.

## Yang ditegakkan mesin

| lapis | pemeriksaan |
|---|---|
| `build.rs` (saat compile) | glyph harus `nf-md-`/`nf-oct-` dan ada di `assets/glyphs.toml`; codepoint tidak boleh berupa karakter kontrol; peringatan untuk glyph yang tidak dipakai |
| `cargo test` | batas 8 warna; konsistensi pemetaan; keluarga + bijeksi; ketetanggaan; batas `dim`; sanitasi output; parsing CLI; satu alfabet Latin untuk seluruh repo (`tests/hygiene.rs`); plus tes yang **sengaja melanggar aturan** untuk membuktikan setiap audit benar-benar melaporkan |
| `ticon --audit` | seluruh pemeriksaan `audit()` dengan laporan; exit 1 bila ada temuan, exit 0 bila bersih |

Contoh laporan bila ada masalah:

```text
ikon: 3 masalah konsistensi di icons.toml:
  - keluarga 'netral' dan 'dokumen' berbagi warna 'blue' — satu warna harus satu keluarga
  - 'doc' dan 'note' bersebelahan di icons.toml tapi beda keluarga ('netral' vs 'dokumen') dengan warna sama 'blue'
  - pemakaian 'dim' mencapai 17 aturan, batasnya 10 — ...
```
