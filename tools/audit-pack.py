#!/usr/bin/env python3
"""Buktikan klaim pack bawaan: "tidak butuh Nerd Font, dan satu sel".

Dua hal yang sering dianggap satu, padahal tidak:

* **Font** — karakter harus ada di font dasar yang diaudit. Tidak ada = kotak,
  persis masalah yang pack ini sollen.
* **Lebar** — karakter dengan `East_Asian_Width = A` (*ambiguous*) bisa jadi dua
  sel di terminal yang dikonfigurasi *ambiguous = lebar*. Tidak ada di sana =
  kotak. Yang `A` = ada di mana pun, tapi bisa merusak kolom. Untuk TUI itu
  kadang lebih buruk dari kotak, jadi ada modus `--sempit`.

Sumber kebenaran dibaca dari `packs/*/pack.json` — data, bukan enum di Rust,
supaya pack yang ditambahkan orang lain ikut diaudit.

Jalankan:

    python tools/audit-pack.py            # semua pack bawaan
    python tools/audit-pack.py --sempit   # gagal kalau ada yang ambiguous

Keluar dengan 1 kalau ada klaim yang tidak terpenuhi; 0 kalau semua hijau.
"""
import argparse
import glob
import json
import os
import sys
import unicodedata

AKAR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
PACKS = os.path.join(AKAR, "packs")

# Font yang tidak harus di-install terpisah di Windows 10/11. Bukan pilihan
# subjektif: daftar ini yang jadi dasar klaim "tidak butuh Nerd Font".
FONTS = [
    ("consolas", r"C:\Windows\Fonts\consola.ttf"),
    ("courier new", r"C:\Windows\Fonts\cour.ttf"),
    ("arial", r"C:\Windows\Fonts\arial.ttf"),
    ("segoe ui", r"C:\Windows\Fonts\segoeui.ttf"),
    ("segoe ui symbol", r"C:\Windows\Fonts\seguisym.ttf"),
    ("cascadia mono", r"C:\Windows\Fonts\CascadiaMono.ttf"),
    ("cascadia code", r"C:\Windows\Fonts\CascadiaCode.ttf"),
]

# Lebar yang aman: `Na` dan `N` selalu satu sel di semua terminal.
AMAN = {"Na", "N"}


def cmap_font(path):
    from fontTools.ttLib import TTFont

    font = TTFont(path, fontNumber=0, lazy=True)
    codes = set()
    for tabel in font["cmap"].tables:
        codes.update(tabel.cmap.keys())
    font.close()
    return codes


def karakter_pack(path):
    """Semua karakter bentuk di satu pack, sebagai (konteks, karakter)."""
    with open(path, encoding="utf-8") as f:
        isi = json.load(f)
    keluar = [(f"shapes.{k}", v) for k, v in (isi.get("shapes") or {}).items()]
    if isi.get("folder_shape"):
        keluar.append(("folder_shape", isi["folder_shape"]))
    return keluar


def main():
    # Peta ikon sengaja memakai karakter non-ASCII, sementara stdout di Windows
    # default-nya cp1252 - jadi mencetak `U+2666` akan gagal dengan
    # UnicodeEncodeError *setelah* audit-nya sendiri selesai. Paksa UTF-8 lebih
    # dulu, dengan `errors="replace"` supaya kode karakternya tetap terlihat
    # kalau konsolnya memang tidak bisa.
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser(description="Audit karakter bentuk pada icon pack")
    parser.add_argument(
        "--sempit",
        action="store_true",
        help="gagal juga kalau ada karakter ambiguous-width (EAW A), bukan cuma yang tidak ada di font",
    )
    parser.add_argument(
        "--hanya",
        metavar="NAMA",
        help="audit hanya pack bernama ini (bisa diulang)",
        action="append",
        default=[],
    )
    args = parser.parse_args()

    try:
        from fontTools.ttLib import TTFont  # noqa: F401
    except ImportError:
        print("fontTools belum ada: pip install fonttools", file=sys.stderr)
        return 1

    cmap = {}
    for nama, path in FONTS:
        if os.path.exists(path):
            cmap[nama] = cmap_font(path)

    pack = sorted(glob.glob(os.path.join(PACKS, "*", "pack.json")))
    if args.hanya:
        pack = [p for p in pack if os.path.basename(os.path.dirname(p)) in args.hanya]
        hilang = set(args.hanya) - {os.path.basename(os.path.dirname(p)) for p in pack}
        if hilang:
            print(f"pack tidak dikenal: {', '.join(sorted(hilang))}", file=sys.stderr)
            return 1
    if not pack:
        print(f"tidak ada pack di {PACKS}", file=sys.stderr)
        return 1

    print(f"font dicek   : {len(cmap)} dari {len(FONTS)}")
    print(f"pack dicek   : {len(pack)}")
    if args.sempit:
        print("mode         : --sempit (EAW harus N atau Na)")
    print()

    gagal = 0
    for path in pack:
        with open(path, encoding="utf-8") as f:
            isi = json.load(f)
        print(f"[{os.path.basename(os.path.dirname(path))}] {isi['pack']['name']} {isi['pack']['version']}")
        print(f"  {'konteks':<16} {'char':<5} {'codepoint':<11} {'EAW':<4} {'di font':<9} status")
        for konteks, bentuk in karakter_pack(path):
            for ch in bentuk:
                codepoint = ord(ch)
                eaw = unicodedata.east_asian_width(ch)
                kurang = sorted(n for n, codes in cmap.items() if codepoint not in codes)
                cakupan = f"{len(cmap) - len(kurang)}/{len(cmap)}" if cmap else "n/a"

                masalah = []
                if cmap and kurang:
                    masalah.append("tidak ada di " + ", ".join(kurang))
                if args.sempit and eaw not in AMAN:
                    masalah.append(f"EAW={eaw}, bisa dua sel")

                if masalah:
                    gagal += 1
                print(
                    f"  {konteks:<16} {ch:<5} U+{codepoint:04X}     {eaw:<4} "
                    f"{cakupan:<9} {'GAGAL: ' + '; '.join(masalah) if masalah else 'OK'}"
                )
        print()

    if gagal:
        print(f"{gagal} klaim pack tidak terpenuhi")
        return 1
    print("SEMUA klaim pack terpenuhi")
    return 0


if __name__ == "__main__":
    sys.exit(main())
