# Kebijakan keamanan

## Melaporkan

Laporkan suspected kerentanan lewat **GitHub Security Advisories** (tab
`Security` → `Report a vulnerability`), bukan issue publik. Saya balas secepat
yang bisa — proyek ini dikelola satu orang, jadi tanpa jam kerja SLA; laporan
yang rapi (langkah reproduksi + dampak) akan selalu didahulukan.

## Apa yang dijaga, dan di mana

| Lapis | Lokasi | Yang dicek |
|---|---|---|
| Output terminal | `src/render.rs` (`sanitize`) | karakter C0/C1 dan pengendali arah bidi (U+202A–202E, U+2066–2069) ditulis sebagai `\u{...}`, tidak pernah dieksekusi terminal |
| Batas muat data | `src/mapping.rs` (`Rules::load_from`) | `icons.toml` ditolak bila memuat karakter kontrol, kode palet non-angka, atau kunci folder yang bentrok |
| Tabel glyph | `build.rs`, `src/glyph.rs` | nama glyph harus dikenal; codepoint tidak boleh karakter kontrol |
| Data upstream | `tools/gen-glyphs.py` | SHA-256 `glyphnames.json` diverifikasi sebelum `assets/glyphs.toml` ditulis |
| CI | `.github/workflows/ci.yml` | action dipin ke commit SHA; `GITHUB_TOKEN` dibatasi `contents: read` |

Perubahan yang menghapus atau melemahkan lapisan mana pun di tabel itu perlu
ditinjau seperti perubahan keamanan, bukan seperti chores biasa.

## Yang bukan kerentanan

- `ikon` hanya membaca: tidak menulis berkas, tidak mengirim apa pun, tidak
  menjalankan apa pun yang ia baca dari isi folder.
- Nama berkas yang memuat karakter kontrol **tidak dieksekusi** — karakter itu
  ditampilkan sebagai `\u{1b}`. Ini perilaku yang diinginkan, bukan kebocoran.
- Laporan untuk versi lama tetap diterima, tapi hanya untuk kerentanan keamanan.

## Versi yang didukung

| Versi | Didukung |
|---|---|
| 0.2.x | ya |
| 0.1.x | hanya kerentanan keamanan |
