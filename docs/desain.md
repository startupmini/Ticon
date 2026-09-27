# Desain

README berisi *apa* yang ditegakkan; halaman ini berisi *mengapa* begitu.
Angka-angkanya: 38 kategori + 26 folder well-known = 64 aturan, 461 ekstensi,
55 glyph, 8 warna.

## Flat: satu keluarga glyph

Semua ikon memakai glyph `nf-md-` (Material Design Icons): satu grid 24px,
satu ketebalan stroke. Satu glyph dari keluarga lain sudah cukup untuk merusak
kesan flat, jadi ini bukan selera — `build.rs` menolak compile.

## Warna per kategori, bukan per tipe file

Kalau `.ts`, `.js`, `.py` masing-masing punya warna, daftar berubah jadi
pelangi logo. Sebaliknya **satu kategori = satu warna**: seluruh bahasa
pemrograman berbagi sian, dan perbedaannya dibawa bentuk glyph (`by_ext`).
Keramaian ditekan di warna, variasi disimpan di bentuk.

## Delapan warna, delapan keluarga

Palet berasal dari 16 warna ANSI terminal; separuhnya milik teks, jadi tinggal
delapan slot. Delapan slot itu diikat ke delapan keluarga **satu-satu**:

| keluarga | warna | aturan | mengapa warna itu |
|---|---|---|---|
| `kode` | cyan | 8 | bahasa, container, cloud, folder sumber — sian dibaca "sumber" |
| `kerja` | green | 6 | shell, test, task — hijau: berjalan, lulus, beres |
| `data` | yellow | 9 | data terstruktur, arsip, lockfile, paket, database |
| `media` | magenta | 10 | gambar, audio, video, slide, model, font — aset visual |
| `dokumen` | blue | 7 | folder bawaan + tulisan: note, readme, pdf, word |
| `aman` | red | 7 | rahasia, kunci, sertifikat, jejak vcs |
| `netral` | white | 8 | teks polos, config, log, biner — tanpa aksen |
| `redup` | dim | 9 | berkas tak dikenal + artefak hasil generate |

Karena keputusan warna hidup di satu tabel `[families]`, "siapa yang berbagi
warna dengan siapa" tidak perlu ditebak dari 64 baris. Dua keluarga berebut
satu warna = gagal test.

Beberapa pemindahan yang disadari saat pembuatan ulang ini:

* `key` dan `cert` pindah kuning → **merah** (bersama `secret` dan `.git` —
  semuanya soal akses);
* `database` pindah merah → **kuning** (SQL/dump itu data, bukan risiko);
* `pdf` pindah merah → **biru** (dokumen; merah disisakan untuk `aman`,
  supaya nama keluarga tidak berbohong);
* `audio`, `slide` → **magenta** (media sungguhan), `sheet` → **kuning**
  (data), `cloud` → **sian** (infra kode), `font` → **magenta** (aset).

## `dim` = "tidak ada yang menarik di sini"

Sebelumnya `dim` menutupi **21 dari 64 aturan** — sepertiga daftar terlihat
mati, dan `white` (satu-satunya aksen terang) cuma dipakai satu aturan. Kini
`dim` tinggal **9 aturan** dan cap-nya mesin, bukan selera:

* `file` — kategori bawaan untuk berkas yang tidak dikenal;
* `cache` — sampah sementara;
* folder artefak hasil generate: `node_modules`, `vendor`, `target`, `build`,
  `dist`, `out`, `.cache`.

Prinsipnya: **yang sengaja diredupkan hanya yang memang tak menarik** — berkas
tak dikenal dan hasil build. Sisanya pindah ke keluarga yang bermakna
(`config`/`license`/`log`/`binary`/`bin` → `netral`, `lock`/`package`/`disk`
→ `data`, `cloud` → `kode`, `font` → `media`). Batasnya `MAX_DIM_RULES = 10`
di `src/mapping.rs`; menaikkannya harus lewat keputusan sadar di sana.

## Ketetanggaan

Aturannya: **dua aturan yang bersebelahan di `icons.toml` hanya boleh sewarna
kalau satu keluarga.** Lintas keluarga yang sewarna = pembagian warna kabur
dan gagal di test `kategori_berdekatan_tidak_silang_keluarga`.

Mengapa tidak absolut (sama sekali tak boleh sewarna)? Karena keluarga dan
bagian file sama-sama dikelompokkan secara semantik, keduanya bertabrakan:
bagian Media berisi `image`, `audio`, `video` — tiga anggota satu keluarga,
mustahil disusun bersebelahan tanpa sewarna kecuali salah satunya diusir ke
keluarga yang salah. Pengecualian "satu keluarga" adalah harga yang dibayar
agar pemengelompokan tetap jujur.

Tujuh pasangan tetangga kategori yang kini sewarna **secara eksplisit**:
`note|readme`, `image|audio`, `audio|video`, `package|disk`,
`container|cloud`, `model|font`, `key|cert` — semuanya satu keluarga, jadi
warna di situ bukan kebetulan.

Pembacaan lain: kalau kamu lebih suka aturan absolut, harganya adalah
menata ulang urutan file supaya anggota satu keluarga tidak pernah
bersebelahan.

## Satu sumber kebenaran untuk warna

Dulu setiap baris menulis `color = "cyan"` sendiri-sendiri; sekarang baris
menulis `family = "kode"` dan warnanya diturunkan lewat `Rules::color_of`.
Keputusannya hanya dibuat sekali di `[families]`, sehingga:

* menambah aturan tidak bisa tanpa memilih keluarga (field-nya wajib);
* dua tempat tidak bisa berbeda — tidak ada tempat kedua;
* `--list` bisa mencetak "siapa berbagi warna" sebagai tabel tersendiri.

Pasangan `family` → warna lalu dipasang di dua lapis: `cargo test` menjelaskan
maksudnya (termasuk dua test yang sengaja melanggar aturan untuk membuktikan
detektornya hidup), `ticon --audit` memeriksanya sebelum rilis.

## Warna mengikuti theme

Palet memakai warna SGR terminal, bukan truecolor hardcoded. Imbalannya: ikon
selaras dengan theme kamu dan ikut berubah saat ganti theme, tanpa konfigurasi.
Biayanya: kamu tidak bisa menentukan warna hex sendiri — itu keputusan sadar,
karena "diam di terminal" lebih diutamakan daripada "persis seperti di gambar".

## Asal data

Codepoint glyph diambil dari glyphnames.json resmi Nerd Fonts v3.4.0 lewat
`tools/gen-glyphs.py`, dan hanya glyph yang dipakai yang masuk repo (55 dari
10.764). Kode dan tabel pemetaan ditulis dari nol — konsepnya umum, salinannya
nol. Rincian di [README](../README.md#asal-data).
