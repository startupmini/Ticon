"""Unduh DepartureMono (Nerd Fonts v3), verifikasi SHA-256, ekstrak varian Mono.

Varian Mono dipilih karena satu lebar untuk ikon dan teks — persis seperti
terminal, dan itu yang dipakai untuk merender pratinjau README.
"""
import hashlib
import io
import json
import lzma
import os
import tarfile
import urllib.request

DEST = os.path.join(os.path.dirname(__file__), "..", ".cache-preview")
BASE = "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1"
NAMA = "DepartureMono.tar.xz"
SHA = "SHA-256.txt"

os.makedirs(DEST, exist_ok=True)


def unduh(url, tujuan):
    if os.path.exists(tujuan):
        return open(tujuan, "rb").read()
    print("mengunduh", url)
    with urllib.request.urlopen(url) as r, open(tujuan, "wb") as f:
        f.write(r.read())
    return open(tujuan, "rb").read()


arsip = unduh(f"{BASE}/{NAMA}", os.path.join(DEST, NAMA))
daftar = unduh(f"{BASE}/{SHA}", os.path.join(DEST, SHA)).decode("utf-8")

# verifikasi hash: baris di SHA-256.txt berbentuk "<hex>  <nama>"
harapan = None
for baris in daftar.splitlines():
    bagian = baris.split()
    if len(bagian) == 2 and bagian[1] == NAMA:
        harapan = bagian[0]
        break
if harapan is None:
    raise SystemExit(f"hash untuk {NAMA} tidak ada di {SHA}")

aktual = hashlib.sha256(arsip).hexdigest()
if aktual != harapan:
    raise SystemExit(f"hash tidak cocok: diharapkan {harapan}, dapat {aktual}")
print("hash cocok:", aktual[:16], "...")

with tarfile.open(fileobj=io.BytesIO(lzma.decompress(arsip))) as tar:
    nama = [m.name for m in tar.getmembers() if m.name.lower().endswith((".ttf", ".otf"))]
    # Varian Mono ditandai "NerdFontMono" pada nama berkasnya.Perhatikan: nama
    # family's sendiri juga mengandung "Mono" (DepartureMono...), jadi tidak
    # bisa sekadar mencari "mono".
    tetap = [n for n in nama if "nerdfontmono" in n.lower()]
    if not tetap:
        tetap = [n for n in nama if "regular" in n.lower()]
    with open(os.path.join(DEST, "font-mono.otf"), "wb") as f:
        f.write(tar.extractfile(tetap[0]).read())
    lis = [n for n in tar.getnames() if "license" in n.lower() or "ofl" in n.lower()]
    if lis:
        with open(os.path.join(DEST, "LICENSE-font.txt"), "wb") as f:
            f.write(tar.extractfile(lis[0]).read())

ukuran = os.path.getsize(os.path.join(DEST, "font-mono.otf"))
print("font:", tetap[0], ukuran, "byte")
print("DATA", json.dumps({"font": tetap[0], "sha256": aktual, "tag": "v3.5.1"}))
