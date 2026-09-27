#!/usr/bin/env python3
"""Bangun `assets/ticon-icons.ttf` - font ikon milik ticon, turunan Nerd Fonts.

Kenapa font sendiri
-------------------

Nerd Fonts menaruh semua ikonnya di codepoint private-use (`U+F0000`-`U+F1AFF`).
Area itu SENGAJA dikosongkan standar Unicode, jadi tidak ada font biasa yang
mengisinya - itulah kenapa terminal tanpa Nerd Font menampilkan kotak. Tidak ada
kode yang bisa memperbaikinya; satu-satunya jalan adalah menyediakan glyph-nya
sendiri.

Kenapa boleh diturunkan
-----------------------

Nerd Fonts berlisensi **SIL Open Font License 1.1** (bukan MIT seperti tertulis
di README upstream - klaim itu salah). OFL 1.1 mengizinkan modifikasi dan
distribusi ulang, dengan syarat:

  1. font turunan **tetap** didistribusikan di bawah OFL 1.1;
  2. teks lisensi ikut disertakan;
  3. tidak boleh dijual sebagai produk mandiri.

Header lisensi upstream **tidak mendeklarasikan Reserved Font Name**, jadi
secara hukum nama lamanya boleh dipakai. Tetap saja font ini diberi nama
sendiri ("Ticon Icons"), karena dua alasan yang lebih penting daripada hukum:
pengguna harus bisa tahu font mana yang mereka pasang, dan tidak ada yang boleh
mengira font ini Nerd Fonts utuh.

Kustomisasi yang dilakukan
--------------------------

Hanya dua, dan sengaja sedikit:

  - **Nama diganti.** Family, subfamily, full name, PostScript name, unique ID.
    Inilah yang membuat font ini bisa dibedakan dari asalnya.
  - **Metadata lisensi diperjelas.** Copyright asli tetap dicantumkan, ditambah
    keterangan bahwa ini turunan.

Bentuk glyphonya **tidak** disentuh sama sekali. Menggambar ulang ikon berarti
mengganti karya Helena Zhang dengan interpretasi kita, dan itu tidak menambah
apa pun bagi pengguna.

Keluaran
--------

  assets/ticon-icons.ttf  - font ikon (55 glyph, sekitar 14 KiB)
  assets/FONT-LICENSE.txt - teks OFL 1.1 dari upstream, wajib ikut
  src/raster_data.rs      - nama glyph -> codepoint (dihasilkan)

Jalankan ulang setiap kali `icons.toml` berubah:

    python tools/gen-font.py
    python tools/gen-font.py --cek   # hanya verifikasi, tanpa menulis
"""
import argparse
import os
import re
import sys
import tarfile

AKAR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
CACHE = os.path.join(AKAR, ".cache-preview")
KELUAR_TTF = os.path.join(AKAR, "assets", "ticon-icons.ttf")
KELUAR_LISENSI = os.path.join(AKAR, "assets", "FONT-LICENSE.txt")
KELUAR_RS = os.path.join(AKAR, "src", "raster_data.rs")

TAG = "v3.5.1"
ARSIP = "DepartureMono.tar.xz"
ARSIP_SHA256 = "7d2d86db20730e26ee4fc926e3c64429d6f9da6fce91e74c325fe1c5ee74d9ee"
FONT_DALAM_ARSIP = "DepartureMonoNerdFontMono-Regular.otf"

KELUARAN = "TiconIcons"
FAMILY = "Ticon Icons"
COPYRIGHT = (
    "Copyright 2022-2024 Helena Zhang (helenazhang.com) for the original glyphs. "
    "Modified for ticon: renamed and subsetted, outlines unchanged. "
    "Licensed under the SIL Open Font License, Version 1.1."
)
LISENSI_RINGKAS = (
    "SIL Open Font License 1.1. This is a renamed subset of DepartureMono "
    "(Nerd Fonts) by Helena Zhang; glyph outlines are unchanged. See "
    "assets/FONT-LICENSE.txt for the full license text."
)


def sha256(path):
    import hashlib

    h = hashlib.sha256()
    with open(path, "rb") as f:
        for blok in iter(lambda: f.read(1 << 20), b""):
            h.update(blok)
    return h.hexdigest()


def glyph_dipakai():
    """Nama glyph yang dirujuk `icons.toml` (semua tabel `by_ext`, bukan cuma satu)."""
    toml = open(os.path.join(AKAR, "icons.toml"), encoding="utf-8").read()
    isi = "\n".join(b for b in toml.splitlines() if not b.lstrip().startswith("#"))
    nama = set(re.findall(r'glyph\s*=\s*"(nf-[^"]+)"', isi))
    for _, tabel in re.findall(
        r"^\[([^\]]*by_ext)\]$(.*?)(?=^\[|\Z)", isi, re.M | re.S
    ):
        nama.update(re.findall(r'"(nf-[a-z0-9_\-]+)"', tabel))
    return nama


def codepoint():
    isi = open(os.path.join(AKAR, "assets", "glyphs.toml"), encoding="utf-8").read()
    return {
        m[0]: int(m[1], 16)
        for m in re.findall(r'"(nf-[^"]+)"\s*=\s*(0x[0-9a-fA-F]+)', isi)
    }


def font_sumber():
    """Font mono dari arsip upstream, diverifikasi lewat SHA-256 arsip."""
    path = os.path.join(CACHE, "font-mono.otf")
    arsip = os.path.join(CACHE, ARSIP)
    if os.path.exists(arsip) and sha256(arsip) != ARSIP_SHA256:
        print(
            "SHA-256 arsip tidak cocok!\n  diharapkan %s\n  ditemukan %s"
            % (ARSIP_SHA256, sha256(arsip)),
            file=sys.stderr,
        )
        return None
    if not os.path.exists(path):
        import subprocess

        subprocess.run(
            [sys.executable, os.path.join(AKAR, "tools", "unduh-font.py")], check=True
        )
        with tarfile.open(arsip, "r:xz") as t:
            with open(path, "wb") as f:
                f.write(t.extractfile(FONT_DALAM_ARSIP).read())
    return path


def buat_font(sumber, codes):
    from fontTools import subset
    from fontTools.ttLib import TTFont

    opsi = subset.Options()
    opsi.layout_features = ["*"]
    opsi.name_IDs = ["*"]
    opsi.notdef_outline = True
    opsi.drop_tables = ["DSIG"]

    font = TTFont(sumber)
    s = subset.Subsetter(options=opsi)
    s.populate(unicodes=sorted(codes))
    s.subset(font)

    # Nama diganti. Inilah satu-satunya kustomisasi yang dilakukan, dan itu
    # disengaja: mengganti outline berarti mengganti karya asli tanpa menambah
    # apa pun bagi pengguna.
    nama = font["name"]
    for id_, nilai in (
        (1, FAMILY),
        (2, "Regular"),
        (3, "%s:1.000" % KELUARAN),
        (4, "%s Regular" % FAMILY),
        (5, "Version 1.000"),
        (6, KELUARAN),
        (13, LISENSI_RINGKAS),
        (14, "https://openfontlicense.org"),
    ):
        nama.setName(nilai, id_, 3, 1, 0x409)
        nama.setName(nilai, id_, 1, 0, 0)
    # Copyright asli tetap dicantumkan - OFL mewajibkannya.
    nama.setName(COPYRIGHT, 0, 3, 1, 0x409)
    nama.setName(COPYRIGHT, 0, 1, 0, 0)

    font["head"].fontRevision = 1.0
    os.makedirs(os.path.dirname(KELUAR_TTF), exist_ok=True)
    font.save(KELUAR_TTF)
    font.close()


def cek_font():
    """Verifikasi font yang sudah ada: nama, lisensi, dan kelengkapan glyph."""
    from fontTools.ttLib import TTFont

    if not os.path.exists(KELUAR_TTF):
        print("%s belum ada; jalankan tanpa --cek dulu" % KELUAR_TTF, file=sys.stderr)
        return 1
    font = TTFont(KELUAR_TTF, lazy=True)
    nama = font["name"]
    family = nama.getDebugName(1) or ""
    ps = nama.getDebugName(6) or ""
    copied = nama.getDebugName(0) or ""
    cmap = set()
    for tabel in font["cmap"].tables:
        cmap.update(tabel.cmap.keys())
    font.close()

    masalah = []
    if family != FAMILY:
        masalah.append("family '%s' seharusnya '%s'" % (family, FAMILY))
    if ps != KELUARAN:
        masalah.append("PostScript name '%s' seharusnya '%s'" % (ps, KELUARAN))
    if "SIL Open Font License" not in copied:
        masalah.append("copyright tidak menyebut OFL")
    if "Helena Zhang" not in copied:
        masalah.append("copyright asli tidak ikut dicantumkan")
    dipakai = codepoint()
    hilang = sorted(n for n in glyph_dipakai() if dipakai[n] not in cmap)
    if hilang:
        masalah.append("glyph hilang dari font: %s" % hilang)
    if not os.path.exists(KELUAR_LISENSI):
        masalah.append("assets/FONT-LICENSE.txt hilang - OFL mewajibkannya ikut")
    else:
        teks = open(KELUAR_LISENSI, encoding="utf-8").read()
        if "SIL OPEN FONT LICENSE" not in teks:
            masalah.append("teks OFL di FONT-LICENSE.txt tidak lengkap")

    if masalah:
        for m in masalah:
            print("  " + m, file=sys.stderr)
        return 1
    print("font    : %s / %s (%.1f KiB)"
          % (family, ps, os.path.getsize(KELUAR_TTF) / 1024))
    print("glyph   : %d codepoint, semua %d glyph yang dipakai icons.toml ada"
          % (len(cmap), len(glyph_dipakai())))
    print("lisensi : OFL 1.1 di name table + assets/FONT-LICENSE.txt")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Bangun font ikon ticon dari Nerd Fonts"
    )
    parser.add_argument(
        "--cek", action="store_true", help="verifikasi font yang ada, jangan tulis"
    )
    args = parser.parse_args()

    if args.cek:
        return cek_font()

    dipakai = glyph_dipakai()
    tabel = codepoint()
    hilang = sorted(dipakai - set(tabel))
    if hilang:
        print(
            "glyph dirujuk icons.toml tapi tidak ada di glyphs.toml: %s" % hilang,
            file=sys.stderr,
        )
        return 1

    sumber = font_sumber()
    if sumber is None:
        return 1

    codes = sorted(tabel[n] for n in dipakai)
    buat_font(sumber, codes)

    with tarfile.open(os.path.join(CACHE, ARSIP), "r:xz") as t:
        lisensi = t.extractfile("LICENSE").read().decode("utf-8", "replace")
    with open(KELUAR_LISENSI, "w", encoding="utf-8", newline="\n") as f:
        f.write(
            "Teks lisensi di bawah disalin apa adanya dari arsip Nerd Fonts %s\n"
            "(%s).\n\n"
            "Font assets/ticon-icons.ttf adalah turunan: nama diganti dan glyph-nya\n"
            "dipangkas, tetapi bentuk glyphonya tidak diubah. OFL 1.1 mewajibkan\n"
            "berkas ini ikut tersebar bersama font-nya.\n\n%s\n%s\n"
            % (TAG, FONT_DALAM_ARSIP, "=" * 70, lisensi)
        )

    baris = [
        "//! DIHASILKAN OTOMATIS - jangan diedit tangan.",
        "//!",
        "//! Nama glyph yang dirujuk `icons.toml` -> codepoint-nya. Dipakai untuk",
        "//! menguji bahwa font di `assets/ticon-icons.ttf` masih lengkap.",
        "//!",
        f"//! Sumber: Nerd Fonts {TAG} (SIL OFL 1.1), {FONT_DALAM_ARSIP}.",
        f"//! SHA-256 arsip: {ARSIP_SHA256}",
        "//! Regenerasi: python tools/gen-font.py",
        "",
        "/// Nama glyph yang dirujuk `icons.toml` -> codepoint-nya, terurut.",
        "pub const GLYPH: &[(&str, u32)] = &[",
    ]
    for nama in sorted(dipakai):
        baris.append('    ("%s", 0x%x),' % (nama, tabel[nama]))
    baris += ["];", ""]
    with open(KELUAR_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(baris))

    print("glyph dipakai    : %d" % len(dipakai))
    print(
        "ticon-icons.ttf  : %.1f KiB (dari font penuh 2,2 MB)"
        % (os.path.getsize(KELUAR_TTF) / 1024)
    )
    print("FONT-LICENSE.txt : %.1f KiB" % (os.path.getsize(KELUAR_LISENSI) / 1024))
    print("raster_data.rs   : %.1f KiB" % (os.path.getsize(KELUAR_RS) / 1024))
    return 0


if __name__ == "__main__":
    sys.exit(main())

