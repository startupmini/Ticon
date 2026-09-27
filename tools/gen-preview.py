"""Buat pratinjau ikon untuk README: subset font + SVG yang mandiri.

Kenapa perlu: glyph Nerd Font ada di codepoint private-use, jadi GitHub dan
crates.io merendernya sebagai kotak. Satu-satunya cara menampilkannya di sana
adalah gambar — dan gambar itu harus membawa font-nya sendiri, karena font
pembaca tidak punya glyph tersebut.

Alur:
  1. ambil baris keluaran `ticon` yang benar-benar berisi ikon,
  2. subset font hanya untuk codepoint yang muncul di baris itu,
  3. tulis `preview.svg` dengan font di-embed sebagai data URI.

Font sumber: `DepartureMonoNerdFontMono-Regular.otf` dari Nerd Fonts v3.5.1
(SIL OFL 1.1). SHA-256 arsip diverifikasi `tools/unduh-font.py`.

Jalankan ulang setelah `icons.toml` berubah:

    python tools/unduh-font.py      # sekali saja
    python tools/gen-preview.py
"""
import base64
import os
import subprocess
import sys

from fontTools import subset
from fontTools.ttLib import TTFont

AKAR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CACHE = os.path.join(AKAR, ".cache-preview")
KELUARAN = os.path.join(AKAR, "preview.svg")
LISENSI = os.path.join(AKAR, "assets", "preview", "LICENSE.txt")


def baris_ikon() -> list[str]:
    """Keluaran ticon yang asli, dengan ikon dan tanpa warna.

    Berkas contoh dibuat di folder sementara supaya repo tidak perlu
    menyimpan berkas demo. Keluarannya dibaca sebagai byte lalu di-decode
    UTF-8 eksplisit: glyph Nerd Font ada di private-use area, yang tidak bisa
    di-decode dengan encoding console Windows (cp1252).
    """
    import tempfile

    # Folder contoh dibuat sementara: repo tidak perlu menyimpan berkas demo,
    # dan paket yang terbit ke crates.io tetap bersih.
    contoh = ["main.go", "app.test.ts", "README.md", "backup.tar.gz", ".env",
              "script.sh", "photo.png", "styles.css", "query.sql", "Cargo.toml"]
    with tempfile.TemporaryDirectory(prefix="ikon-sampel-") as s:
        for nama in contoh:
            open(os.path.join(s, nama), "w", encoding="utf-8").close()
        for nama in ("src", "node_modules"):
            os.mkdir(os.path.join(s, nama))
        try:
            hasil = subprocess.run(
                [
                    "cargo", "run", "--quiet", "--",
                    "-1", "--icons", "always", "--color", "never", s,
                ],
                check=True, capture_output=True, cwd=AKAR,
            )
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            print("gagal menjalankan ticon:", e, file=sys.stderr)
            return []
    teks = hasil.stdout.decode("utf-8", errors="replace")
    return [l for l in teks.splitlines() if l.strip()]


def subsetkan(sumber: str, baris: list[str], ttf: str, woff2: str) -> int:
    """Potong font hanya untuk codepoint yang dipakai baris contoh."""
    punya = set(range(0x20, 0x7F))  # ASCII printable: nama berkas, garis bawah
    for l in baris:
        punya.update(ord(c) for c in l)
    codes = sorted(punya)

    for keluar, flavor in ((ttf, None), (woff2, "woff2")):
        opsi = subset.Options()
        opsi.layout_features = ["*"]
        opsi.name_IDs = ["*"]
        opsi.notdef_outline = True
        opsi.drop_tables = ["DSIG"]
        font = TTFont(sumber)
        s = subset.Subsetter(options=opsi)
        s.populate(unicodes=codes)
        s.subset(font)
        if flavor:
            font.flavor = flavor
        font.save(keluar)
    return len(codes)


def svg(baris: list[str], woff2: str) -> str:
    """SVG mandiri: font di-embed sebagai data URI, teks tetap teks."""
    data = base64.b64encode(open(woff2, "rb").read()).decode("ascii")
    tinggi_baris, lebar, tepi = 30, 700, 24
    tinggi = 26 + tinggi_baris * len(baris)
    isi = []
    for i, l in enumerate(baris):
        teks = l.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
        isi.append(f'  <text x="{tepi}" y="{36 + i * tinggi_baris}">{teks}</text>')
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" '
        f'width="{lebar}" height="{tinggi}" viewBox="0 0 {lebar} {tinggi}" '
        'role="img" aria-label="Contoh keluaran ticon">\n'
        "  <title>Contoh keluaran ticon</title>\n"
        "  <defs><style>\n"
        "    @font-face { font-family: 'PreviewNerd'; "
        f"src: url(data:font/woff2;base64,{data}) format('woff2'); }}\n"
        "    text { font-family: 'PreviewNerd', monospace; font-size: 17px; fill: #c9d1d9; }\n"
        "  </style></defs>\n"
        '  <rect width="' + str(lebar) + '" height="' + str(tinggi) + '" fill="#0d1117"/>\n'
        + "\n".join(isi)
        + "\n</svg>\n"
    )


def main() -> None:
    sumber = os.path.join(CACHE, "font-mono.otf")
    if not os.path.exists(sumber):
        raise SystemExit("font belum ada; jalankan `python tools/unduh-font.py` dulu")
    baris = baris_ikon()
    if not baris:
        raise SystemExit("tidak ada baris keluaran untuk dijadikan pratinjau")

    os.makedirs(os.path.dirname(LISENSI), exist_ok=True)
    if not os.path.exists(LISENSI):
        with open(LISENSI, "wb") as f:
            f.write(open(os.path.join(CACHE, "LICENSE-font.txt"), "rb").read())

    ttf = os.path.join(CACHE, "subset.ttf")
    woff2 = os.path.join(CACHE, "subset.woff2")
    n = subsetkan(sumber, baris, ttf, woff2)
    with open(KELUARAN, "w", encoding="utf-8", newline="\n") as f:
        f.write(svg(baris, woff2))

    print(f"baris contoh : {len(baris)}")
    print(f"codepoint    : {n}")
    print(f"subset woff2 : {os.path.getsize(woff2) / 1024:.1f} KiB")
    print(f"preview.svg  : {os.path.getsize(KELUARAN) / 1024:.1f} KiB")
    print(f"ttf raster   : {ttf}")


if __name__ == "__main__":
    main()
