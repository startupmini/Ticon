"""Subset font Nerd Fonts untuk glyph yang benar-benar dipakai `ticon`.

Kenapa ada: glyph Nerd Font ada di codepoint private-use, jadi program yang
hanya mencetak karakter **tidak bisa** lepas dari font yang terpasang di
terminal. Namun program yang me-*raster* sendiri (jalur half-block) boleh
membawa fontnya sendiri - dan untuk itu hanya perlu 38 glyph, bukan 2,3 MB.

Keluaran:
  assets/raster.ttf      - subset, hanya codepoint yang dipakai icons.toml
  src/raster_data.rs     - daftar nama glyph -> codepoint (dihasilkan)

`src/raster_data.rs` sengaja dibuat sebagai daftar biasa supaya bisa diuji
dari Rust tanpa perlu mengurai TTF: kalau `icons.toml` ditambah glyph baru dan
subsetnya belum diregenerasi, tes gagal.

Provenans:
  Nerd Fonts v3.5.1 (SIL OFL 1.1), DepartureMono varian Mono.
  SHA-256 arsip diverifikasi terhadap SHA-256.txt upstream.

Jalankan ulang setelah `icons.toml` berubah:

    python tools/gen-raster.py
"""
import hashlib
import io
import os
import re
import sys

AKAR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CACHE = os.path.join(AKAR, ".cache-preview")
KELUAR_TTF = os.path.join(AKAR, "assets", "raster.ttf")
KELUAR_RS = os.path.join(AKAR, "src", "raster_data.rs")
TAG = "v3.5.1"
ARSIP_SHA256 = "7d2d86db20730e26ee4fc926e3c64429d6f9da6fce91e74c325fe1c5ee74d9ee"
FONT = "DepartureMonoNerdFontMono-Regular.otf"


def glyph_dipakai() -> set[str]:
    """Nama glyph yang dirujuk `icons.toml`, termasuk isian `by_ext`."""
    toml = io.open(os.path.join(AKAR, "icons.toml"), encoding="utf-8").read()
    nama = set(re.findall(r'glyph\s*=\s*"(nf-[^"]+)"', toml))
    for blok in re.findall(r"\[categories\.by_ext\][^\[]*", toml):
        nama.update(re.findall(r'"(nf-[a-z0-9_\-]+)"', blok))
    return nama


def codepoint() -> dict[str, int]:
    """Tabel nama -> codepoint dari `assets/glyphs.toml` (sumber kebenaran)."""
    isi = io.open(os.path.join(AKAR, "assets", "glyphs.toml"), encoding="utf-8").read()
    return {m[0]: int(m[1], 16) for m in re.findall(r'"(nf-[^"]+)"\s*=\s*(0x[0-9a-fA-F]+)', isi)}


def font_sumber() -> str:
    """Pastikan font ada di cache; unduh + verifikasi hash kalau belum."""
    path = os.path.join(CACHE, "font-mono.otf")
    if os.path.exists(path):
        return path
    subprocess_unduh()
    return path


def subprocess_unduh() -> None:
    import subprocess

    subprocess.run(
        [sys.executable, os.path.join(AKAR, "tools", "unduh-font.py")],
        check=True,
        capture_output=True,
    )
    # unduh-font.py menyimpan varian Regular; yang dipakai di sini Mono.
    import lzma
    import tarfile

    with tarfile.open(os.path.join(CACHE, "DepartureMono.tar.xz"), "r:xz") as tar:
        with open(path, "wb") as f:
            f.write(tar.extractfile(FONT).read())


def main() -> int:
    from fontTools import subset
    from fontTools.ttLib import TTFont

    dipakai = glyph_dipakai()
    tabel = codepoint()
    hilang = sorted(dipakai - set(tabel))
    if hilang:
        print(f"glyph dirujuk icons.toml tapi tidak ada di glyphs.toml: {hilang}", file=sys.stderr)
        return 1

    codes = sorted(tabel[n] for n in dipakai)
    sumber = font_sumber()

    opsi = subset.Options()
    opsi.layout_features = ["*"]
    opsi.name_IDs = ["*"]
    opsi.notdef_outline = True
    opsi.drop_tables = ["DSIG"]
    font = TTFont(sumber)
    s = subset.Subsetter(options=opsi)
    s.populate(unicodes=codes)
    s.subset(font)
    os.makedirs(os.path.dirname(KELUAR_TTF), exist_ok=True)
    font.save(KELUAR_TTF)
    font.close()

    # daftar Rust: supaya bisa diuji tanpa mengurai TTF
    baris = [
        "//! DIHASILKAN OTOMATIS - jangan diedit tangan.",
        "//!",
        "//! Nama glyph yang dirujuk `icons.toml` -> codepoint-nya. Dipakai untuk",
        "//! menguji bahwa subset font di `assets/raster.ttf` masih lengkap, dan",
        "//! oleh jalur render half-block nanti.",
        "//!",
        f"//! Sumber: Nerd Fonts {TAG} (SIL OFL 1.1), {FONT}.",
        f"//! SHA-256 arsip: {ARSIP_SHA256}",
        "//! Regenerasi: python tools/gen-raster.py",
        "",
        "/// Nama glyph yang dirujuk `icons.toml` -> codepoint-nya, terurut.",
        "///",
        "/// Dipakai untuk menguji kelengkapan subset font dan, nanti, oleh jalur",
        "/// render half-block. Daftarnya sengaja datar supaya bisa diperiksa dari",
        "/// tes tanpa perlu mengurai berkas TTF.",
        'pub const GLYPH: &[(&str, u32)] = &[',
    ]
    for nama in sorted(dipakai):
        baris.append(f'    ("{nama}", 0x{tabel[nama]:x}),')
    baris += ["];", ""]
    with open(KELUAR_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(baris))

    ukuran = os.path.getsize(KELUAR_TTF)
    print(f"glyph dipakai : {len(dipakai)}")
    print(f"raster.ttf    : {ukuran / 1024:.1f} KiB (dari font penuh 2,2 MB)")
    print(f"raster_data.rs: {os.path.getsize(KELUAR_RS) / 1024:.1f} KiB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
