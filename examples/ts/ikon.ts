// Resolver referensi untuk kontrak `ticon-map/2` dalam TypeScript.
//
// Tiga hal yang dijaga di sini:
//   1. Urutan resolver diambil dari DATA (`order` di peta), bukan ditulis di
//      kode — jadi tidak perlu menebak tahap mana yang menang lebih dulu.
//   2. `color` boleh diabaikan; yang dipakai adalah `family`, yang dipetakan
//      ke gaya aplikasi sendiri lewat `Tema` di bawah.
//   3. `width` dan `fallback` **hanya ada kalau diisi** — jadi `undefined`
//      berarti "tidak diketahui", bukan "nol" atau "?".
//
// Bukti kebenaran: `cek.ts` membandingkan keluaran fungsi ini dengan
// `tests/konformasi/harapan.tsv` yang sama dipakai tes Rust.

export type Kind = "name" | "suffix" | "prefix" | "ext" | "dir" | "default";

export type Aturan = {
  kind: Kind;
  priority: number | null;
  key: string;
  family: string;
  color: string;
  glyph: string;
  codepoint: number | null;
  width?: number;
  fallback?: string;
};

type Bawaan = {
  glyph: string;
  codepoint: number | null;
  family: string;
  color: string;
  width?: number;
  fallback?: string;
};

export type PetaIkon = {
  schema: string;
  width_default: number;
  order: Kind[];
  families: Record<string, string>;
  defaults: { file: Bawaan; dir: Bawaan };
  rules: Aturan[];
};

export type Hasil = {
  ch: string;
  fallback: string;
  width: number;
  family: string;
  color: string;
  kind: Kind;
  pattern: string;
};

/** Peta nama keluarga ke gaya milik aplikasi ini. Boleh diganti sepuasnya. */
export class Tema {
  // Ditulis tanpa parameter property: sintaks itu tidak bisa di-"type-strip"
  // oleh Node, dan contoh ini harus tetap jalan tanpa build step.
  private readonly gaya: Record<string, string>;

  constructor(gaya: Record<string, string>) {
    this.gaya = gaya;
  }

  gayaUntuk(family: string): string {
    return this.gaya[family] ?? "netral";
  }
}

type Indeks = {
  nama: Map<string, Aturan>;
  awalan: Aturan[];
  akhiran: Aturan[];
  ext: Map<string, Aturan>;
  dir: Map<string, Aturan>;
  bawaan: PetaIkon["defaults"];
  widthDefault: number;
  order: Kind[];
};

export function indeks(peta: PetaIkon): Indeks {
  const nama = new Map<string, Aturan>();
  const awalan: Aturan[] = [];
  const akhiran: Aturan[] = [];
  const ext = new Map<string, Aturan>();
  const dir = new Map<string, Aturan>();
  for (const a of peta.rules) {
    if (a.kind === "name") nama.set(a.key, a);
    else if (a.kind === "prefix") awalan.push(a);
    else if (a.kind === "suffix") akhiran.push(a);
    else if (a.kind === "ext") ext.set(a.key, a);
    else if (a.kind === "dir") dir.set(a.key, a);
  }
  // Yang terpanjang dulu, supaya "readme" menang atas awalan yang lebih umum.
  const urut = (a: Aturan, b: Aturan) => b.key.length - a.key.length;
  awalan.sort(urut);
  akhiran.sort(urut);
  return {
    nama,
    awalan,
    akhiran,
    ext,
    dir,
    bawaan: peta.defaults,
    widthDefault: peta.width_default,
    order: peta.order,
  };
}

function jadi(
  a: Aturan | undefined,
  bawaan: Bawaan,
  kind: Kind,
  pattern: string,
  widthDefault: number,
): Hasil {
  // Tanpa aturan yang cocok, glyph dan warnanya datang dari kategori bawaan.
  // `width`/`fallback` yang tidak ada berarti tidak diketahui, jadi nilai
  // bawaan yang dipakai — bukan angka tebakan.
  const codepoint = a?.codepoint ?? bawaan.codepoint;
  return {
    ch: codepoint != null ? String.fromCodePoint(codepoint) : "?",
    fallback: a?.fallback ?? bawaan.fallback ?? "?",
    width: a?.width ?? bawaan.width ?? widthDefault,
    family: a?.family ?? bawaan.family,
    color: a?.color ?? bawaan.color,
    kind,
    pattern,
  };
}

/** Ikon untuk nama folder. */
export function ikonFolder(ix: Indeks, nama: string): Hasil {
  const key = nama.toLowerCase();
  const a = ix.dir.get(key);
  return jadi(a, ix.bawaan.dir, a ? "dir" : "default", a ? key : "", ix.widthDefault);
}

/** Ikon untuk nama berkas, memakai urutan dari data. */
export function ikonBerkas(ix: Indeks, nama: string): Hasil {
  const key = nama.toLowerCase();
  for (const tahap of ix.order) {
    switch (tahap) {
      case "name": {
        const a = ix.nama.get(key);
        if (a) return jadi(a, ix.bawaan.file, "name", key, ix.widthDefault);
        break;
      }
      case "suffix": {
        const a = ix.akhiran.find((r) => key.endsWith(r.key));
        if (a) return jadi(a, ix.bawaan.file, "suffix", a.key, ix.widthDefault);
        break;
      }
      case "prefix": {
        const a = ix.awalan.find((r) => key.startsWith(r.key));
        if (a) return jadi(a, ix.bawaan.file, "prefix", a.key, ix.widthDefault);
        break;
      }
      case "ext": {
        // Dari titik yang paling kiri: ".tar.gz" menang atas ".gz".
        for (let i = key.indexOf("."); i >= 0; i = key.indexOf(".", i + 1)) {
          const a = ix.ext.get(key.slice(i));
          if (a) return jadi(a, ix.bawaan.file, "ext", key.slice(i), ix.widthDefault);
        }
        break;
      }
      case "default": {
        // `default` adalah hasil akhir, bukan tahap: keluar dari sini.
        return jadi(undefined, ix.bawaan.file, "default", "", ix.widthDefault);
      }
    }
  }
  return jadi(undefined, ix.bawaan.file, "default", "", ix.widthDefault);
}
