#!/usr/bin/env python3
"""Regenerasi assets/glyphs.toml dari data resmi Nerd Fonts.

Kenapa ada langkah build manual begini:

  * Data glyph berasal dari upstream (Nerd Fonts), bukan diketik tangan, jadi
    codepoint-nya tidak bisa salah ketik.
  * Yang di-commit hanya glyph yang benar-benar dipakai `icons.toml`. Tabel
    upstream berisi 10.764 glyph; kita perlu sekitar 50. Memangkasnya berarti
    binary tetap kecil dan tidak ada data mati di repo.
  * Hasilnya di-commit, jadi `cargo build` tidak pernah butuh jaringan.

Pakai:
    python tools/gen-glyphs.py            # pakai cache kalau ada
    python tools/gen-glyphs.py --refresh  # paksa unduh ulang dari upstream
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import urllib.request
from pathlib import Path

NERD_FONTS_VERSION = "3.4.0"
UPSTREAM = (
    f"https://raw.githubusercontent.com/ryanoasis/nerd-fonts/"
    f"v{NERD_FONTS_VERSION}/glyphnames.json"
)

# SHA-256 payload untuk tag di UPSTREAM (534.871 byte). Tag yang ditimpa
# upstream — atau jaringan yang dikompromi — bikin proses berhenti jauh
# sebelum glyphs.toml ditulis. Perbarui hash ini hanya bersamaan dengan
# kenaikan NERD_FONTS_VERSION, setelah memeriksa diff upstream.
EXPECTED_SHA256 = "e2d10d23f5bff0bd6f0676e9b01d9789fcdc656de7b498a2955c27716ea4439c"
TIMEOUT = 30

ROOT = Path(__file__).resolve().parent.parent
ICONS_TOML = ROOT / "icons.toml"
OUTPUT = ROOT / "assets" / "glyphs.toml"
CACHE = ROOT / ".cache" / "glyphnames.json"

HEADER = """\
# DIHASILKAN OTOMATIS — jangan diedit tangan.
#
# Sumber : Nerd Fonts v{version} (MIT) — https://www.nerdfonts.com
# Isi    : hanya glyph yang direferensikan icons.toml.
# Regenerasi: python tools/gen-glyphs.py
#
# Nama memakai awalan `nf-` supaya sama dengan cheat sheet Nerd Fonts.

[glyphs]
"""


def verify(raw: bytes, asal: str) -> None:
    """Tolak payload yang tidak persis seperti yang di-commit saat pin dibuat."""
    aktual = hashlib.sha256(raw).hexdigest()
    if aktual != EXPECTED_SHA256:
        raise SystemExit(
            f"SHA-256 {asal} tidak cocok — payload berbeda dari yang dipin:\n"
            f"  diharapkan {EXPECTED_SHA256}\n"
            f"  didapat    {aktual}\n"
            f"(kalau cache lama, jalankan --refresh setelah memeriksa upstream)"
        )


def fetch(refresh: bool) -> dict:
    if CACHE.exists() and not refresh:
        raw = CACHE.read_bytes()
        verify(raw, f"cache {CACHE.relative_to(ROOT)}")
        return json.loads(raw.decode("utf-8"))
    print(f"mengunduh {UPSTREAM}", file=sys.stderr)
    with urllib.request.urlopen(UPSTREAM, timeout=TIMEOUT) as response:
        raw = response.read()
    verify(raw, "unduhan upstream")
    CACHE.parent.mkdir(parents=True, exist_ok=True)
    CACHE.write_bytes(raw)
    return json.loads(raw.decode("utf-8"))


def used_glyph_names() -> list[str]:
    """Semua nama glyph yang direferensikan icons.toml.

    Hanya sisi kanan tanda `=` yang dibaca, dan komentar dibuang lebih dulu.
    Tanpa itu, contoh nama glyph di dalam komentar ikut terbaca sebagai
    referensi dan dilaporkan sebagai glyph yang hilang.
    """
    names: set[str] = set()
    for line in ICONS_TOML.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0]
        rhs = line.partition("=")[2]
        for value in re.findall(r'"([a-z0-9_-]+)"', rhs):
            if value.startswith("nf-"):
                names.add(value)
    return sorted(names)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--refresh", action="store_true", help="unduh ulang dari upstream")
    args = parser.parse_args()

    data = fetch(args.refresh)
    version = data.get("METADATA", {}).get("version", NERD_FONTS_VERSION)
    names = used_glyph_names()

    lines: list[str] = []
    missing: list[str] = []
    for name in names:
        # Upstream memakai kunci tanpa awalan "nf-".
        key = name[3:] if name.startswith("nf-") else name
        entry = data.get(key)
        if entry is None:
            missing.append(name)
            continue
        lines.append(f'"{name}" = 0x{entry["code"]}')

    if missing:
        print("glyph berikut tidak ada di Nerd Fonts v%s:" % version, file=sys.stderr)
        for name in missing:
            print(f"  - {name}", file=sys.stderr)
        return 1

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(HEADER.format(version=version) + "\n".join(lines) + "\n")
    print(
        f"{len(lines)} glyph ditulis ke {OUTPUT.relative_to(ROOT)} (Nerd Fonts v{version})",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
