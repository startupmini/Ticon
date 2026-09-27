"""Buktikan klaim "bentuk cadangan tidak butuh Nerd Font".

Membaca karakter bentuk dari `src/render.rs`, lalu mengeceknya terhadap font
yang biasa ada di mesin ini. Gagal (exit 1) kalau ada bentuk yang tidak ada di
salah satu font — karena itu persis kejadian yang membuat glyph jadi kotak.

Jalankan ulang setiap kali bentuk di `src/render.rs` berubah:

    python tools/audit-font.py
"""
import os
import re
import sys

AKAR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
RENDER = os.path.join(AKAR, "src", "render.rs")
DIR_FONT = os.environ.get("WINDIR", "C:/Windows") + "/Fonts"

# Font yang dianggap "biasa ada". Kalau satu pun tidak ada di mesin ini,
# pemeriksaan dilewati untuk font itu saja — bukan berarti lolos.
FONTS = [
    "consola.ttf", "cour.ttf", "arial.ttf",
    "segoeui.ttf", "seguisym.ttf", "CascadiaMono.ttf", "CascadiaCode.ttf",
]


def bentuk_dari_sumber() -> set[str]:
    """Semua karakter yang dipakai sebagai bentuk cadangan di render.rs."""
    src = open(RENDER, encoding="utf-8").read()
    karakter = set()
    for blok in re.findall(r"=> '\\u\{([0-9A-Fa-f]+)\}'", src):
        karakter.add(chr(int(blok, 16)))
    if not karakter:
        sys.exit("tidak ada bentuk '\\u{...}' yang ditemukan di src/render.rs")
    return karakter


def main() -> int:
    try:
        from fontTools.ttLib import TTFont
    except ImportError:
        print("fontTools belum ada: pip install fonttools", file=sys.stderr)
        return 1

    bentuk = bentuk_dari_sumber()
    font_ada, font_hilang = [], []
    for nama in FONTS:
        path = os.path.join(DIR_FONT, nama)
        if not os.path.exists(path):
            font_hilang.append(nama)
            continue
        font_ada.append(nama)
        cmap = set()
        font = TTFont(path, fontNumber=0, lazy=True)
        for tabel in font["cmap"].tables:
            cmap.update(tabel.cmap.keys())
        font.close()
        kurang = sorted(c for c in bentuk if ord(c) not in cmap)
        if kurang:
            print(f"GAGAL: {nama} tidak punya {['U+%04X' % ord(c) for c in kurang]}")
            return 1

    print(f"bentuk diperiksa : {len(bentuk)} ({' '.join(sorted('U+%04X' % ord(c) for c in bentuk))})")
    print(f"font dicek       : {', '.join(font_ada) or '(tidak ada)'}")
    if font_hilang:
        print(f"font dilewati    : {', '.join(font_hilang)}")
    print("SEMUA bentuk ada di semua font yang dicek")
    return 0


if __name__ == "__main__":
    sys.exit(main())
