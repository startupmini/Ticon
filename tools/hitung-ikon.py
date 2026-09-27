"""Berapa banyak ikon yang benar-benar bisa dipakai TANPA install font?

Pertanyaan aslinya: "tujuan saya membuat ini karena ingin membuat lebih banyak
font di terminal". Kalau jawabannya cuma 14 karakter, maka tidak layak. Jadi
mari dihitung, bukan ditebak.

Yang diukur:
  - karakter yang ada di font dasar (irisannya semua font = paling aman)
  - karakter yang TIDAK di private-use (kalau PUA, berarti butuh Nerd Font)
  - lebar: EAW N/Na = pasti satu sel; A = bisa dua sel di terminal CJK
  - yang masuk kategori "bisa jadi ikon": simbol, bukan huruf, bukan markup

Hasilnyaangler dipakai untuk memutuskan: perbesar set ikon, atau ransom tulis
font sendiri.
"""
import glob
import os
import sys
import unicodedata as u

from fontTools.ttLib import TTFont

FONTS = {
    "consolas": r"C:\Windows\Fonts\consola.ttf",
    "courier new": r"C:\Windows\Fonts\cour.ttf",
    "arial": r"C:\Windows\Fonts\arial.ttf",
    "segoe ui": r"C:\Windows\Fonts\segoeui.ttf",
    "segoe ui symbol": r"C:\Windows\Fonts\seguisym.ttf",
    "cascadia mono": r"C:\Windows\Fonts\CascadiaMono.ttf",
    "cascadia code": r"C:\Windows\Fonts\CascadiaCode.ttf",
}

AMAN = {"Na", "N"}


def pua(k):
    return (
        0xE000 <= k <= 0xF8FF
        or 0xF0000 <= k <= 0xFFFFD
        or 0x100000 <= k <= 0x10FFFD
    )


def bisa_jadi_ikon(ch):
    """Simbol yang layak jadi ikon, bukan huruf dan bukan emoji."""
    kode = ord(ch)
    if u.category(ch)[0] not in "SPN":  # simbol, penanda, angka
        return False
    if u.category(ch).startswith("So"):  # emoji, symbol lain
        return False
    # buang yang secara visual akan jadi derau: spasi, tanda baca, kurung, operator
    if ch.isspace() or ch in "()[]{}<>|/\\+=~$^`*\"'#@%&,;:?!":
        return False
    nama = u.name(ch, "")
    if any(k in nama for k in ("SPACE", "QUOTATION", "BRACKET", "PARENTHES", "ASTERISK",
                               "NUMBER SIGN", "AMPERSAND", "SEMICOLON", "COLON",
                               "QUESTION", "EXCLAMATION", "COMMA", "FULL STOP",
                               "HYPHEN", "PLUS", "EQUAL", "TILDE", "GRAVE",
                               "CIRCUMFLEX", "SOLIDUS", "BACKSLASH", "DOLLAR",
                               "PERCENT", "AT SIGN")):
        return False
    return True


def main():
    cmap = {}
    for nama, path in FONTS.items():
        if not os.path.exists(path):
            continue
        font = TTFont(path, fontNumber=0, lazy=True)
        codes = set()
        for tabel in font["cmap"].tables:
            codes.update(tabel.cmap.keys())
        font.close()
        cmap[nama] = codes

    semua = set.intersection(*cmap.values()) if cmap else set()
    non_pua = {k for k in semua if not pua(k)}
    sempit = {k for k in non_pua if u.east_asian_width(chr(k)) in AMAN}
    ikon_sempit = {k for k in sempit if bisa_jadi_ikon(chr(k))}
    ikon_ambigu = {
        k for k in non_pua if u.east_asian_width(chr(k)) == "A" and bisa_jadi_ikon(chr(k))
    }

    print("font dicek            : %d" % len(cmap))
    for nama, codes in sorted(cmap.items()):
        print("  %-18s %5d codepoint" % (nama, len(codes)))
    print()
    print("Irisan SEMUA 7 font   : %d" % len(semua))
    print("  ... non private-use : %d" % len(non_pua))
    print("  ... dan EAW N/Na    : %d" % len(sempit))
    print()
    print("LAYAK jadi ikon (bukan huruf, bukan emoji, bukan tanda baca):")
    print("  ada di 7/7 font, EAW N/Na : %3d" % len(ikon_sempit))
    print("  ada di 7/7 font, EAW A    : %3d" % len(ikon_ambigu))
    print()
    print("ANGKA YANG LEBIH REALISTIS - terminal biasanya cuma pakai 1-2 font,")
    print("jadi 'ada di semua' terlalu ketat. Kalau cukup ada di n dari 7:")
    print()
    print("  minimal n font |  semua  | non-PUA | layak ikon | layak+1 sel")
    for n in range(7, 2, -1):
        semua_n = {k for k in range(0x110000) if sum(k in c for c in cmap.values()) >= n}
        npua = {k for k in semua_n if not pua(k)}
        ikon = {k for k in npua if bisa_jadi_ikon(chr(k))}
        satu = {k for k in ikon if u.east_asian_width(chr(k)) in AMAN}
        print("       %d/7       |  %5d |  %5d  |  %5d   |  %5d"
              % (n, len(semua_n), len(npua), len(ikon), len(satu)))
    print()
    print("Contoh 40 karakter aman (7/7 font, EAW N/Na):")
    print("  " + " ".join(chr(k) for k in sorted(ikon_sempit)[:40]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
