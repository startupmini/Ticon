// Bukti konformasi untuk resolver TypeScript: hasil resolver ini harus sama
// persis dengan `tests/konformasi/harapan.tsv`, yang juga dipakai tes Rust.
//
// Pakai:
//   ticon --export=json > icons.json
//   node --experimental-strip-types examples/ts/cek.ts icons.json \
//     tests/konformasi/nama.txt tests/konformasi/harapan.tsv
//
// Kalau selisih muncul, bedanya dicetak baris demi baris.

import { readFileSync } from "node:fs";

import { indeks, ikonBerkas, ikonFolder, type PetaIkon } from "./ikon.ts";

const [berkasPeta, berkasNama, berkasHarapan] = process.argv.slice(2);
if (!berkasPeta || !berkasNama || !berkasHarapan) {
  console.error("pakai: cek.ts <icons.json> <nama.txt> <harapan.tsv>");
  process.exit(2);
}

const peta: PetaIkon = JSON.parse(readFileSync(berkasPeta, "utf8"));
const ix = indeks(peta);

const baris: string[] = readFileSync(berkasNama, "utf8")
  .split("\n")
  .map((b) => b.trim())
  .filter((b) => b.length > 0 && !b.startsWith("#"))
  .map((entri) => {
    const folder = entri.startsWith("dir:");
    const nama = folder ? entri.slice(4) : entri;
    const h = folder ? ikonFolder(ix, nama) : ikonBerkas(ix, nama);
    const cp = h.ch === "?" ? "-" : "0x" + h.ch.codePointAt(0)!.toString(16);
    return [entri, h.kind, h.pattern, h.color, cp].join("\t");
  });

const sekarang = baris.join("\n") + "\n";
// End-of-line dinormalkan: checkout di Windows bisa mengubahnya jadi CRLF, dan
// itu bukan divergensi isi yang mau diuji.
const harapan = readFileSync(berkasHarapan, "utf8").replaceAll("\r\n", "\n");

if (sekarang === harapan) {
  console.log(`konformasi TypeScript: ${baris.length} entri cocok dengan korpus`);
  process.exit(0);
}

const a = sekarang.split("\n");
const b = harapan.split("\n");
console.error("konformasi gagal: resolver TypeScript menyimpang dari korpus");
for (let i = 0; i < Math.max(a.length, b.length); i++) {
  if (a[i] !== b[i]) {
    console.error(`  baris ${i + 1}\n    TS    : ${a[i]}\n    korpus: ${b[i]}`);
  }
}
process.exit(1);
