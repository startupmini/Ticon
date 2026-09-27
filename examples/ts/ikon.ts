// Resolver referensi untuk `ticon-map/1` dalam TypeScript.
//
// Atribut penting di sini:
//   1. Urutancomes dari DATA (`urutan` di icons.json), bukan ditulis di kode —
//      jadi tidak perlu menebak tahap mana yang menang lebih dulu.
//   2. `warna` boleh diabaikan; yang dipakai adalah `keluarga`, yang dipetakan
//      ke gaya aplikasi sendiri lewat `Tema` di bawah.
//   3. `lebar` dan `fallback` ikut dibawa, sehingga aplikasi tanpa Nerd Font
//      tetap bisa merender sesuatu yang berarti.
//
// Bukti kebenaran: `cek.ts` membandingkan keluaran fungsi ini dengan
// `tests/konformasi/harapan.tsv` yang sama dipakai tes Rust.

export type Jenis = "nama" | "akhiran" | "awalan" | "ext" | "dir" | "bawaan";

export type Aturan = {
  jenis: Jenis;
  priority: number | null;
  kunci: string;
  keluarga: string;
  warna: string;
  glyph: string;
  codepoint: number | null;
  lebar: number;
  fallback: string;
};

export type PetaIkon = {
  schema: string;
  lebar_default: number;
  urutan: Jenis[];
  keluarga: Record<string, string>;
  bawaan: {
    berkas: { glyph: string; codepoint: number | null; keluarga: string; warna: string; fallback: string };
    folder: { glyph: string; codepoint: number | null; keluarga: string; warna: string; fallback: string };
  };
  aturan: Aturan[];
};

export type Hasil = {
  ch: string;
  fallback: string;
  lebar: number;
  keluarga: string;
  warna: string;
  jenis: Jenis;
  pola: string;
};

/** Peta nama warna `ticon` ke gaya milik aplikasi ini. Boleh diganti sepuasnya. */
export class Tema {
  // Ditulis tanpa parameter property: sintaks itu tidak bisa di-"type-strip"
  // oleh Node, dan contoh ini harus tetap jalan tanpa build step.
  private readonly gaya: Record<string, string>;

  constructor(gaya: Record<string, string>) {
    this.gaya = gaya;
  }

  gayaUntuk(keluarga: string): string {
    return this.gaya[keluarga] ?? "netral";
  }
}

type Indeks = {
  nama: Map<string, Aturan>;
  awalan: Aturan[];
  akhiran: Aturan[];
  ext: Map<string, Aturan>;
  dir: Map<string, Aturan>;
  /** Family -> warna bawaan; dipakai kalau `keluarga` tidak ada. */
  bawaan: PetaIkon["bawaan"];
  lebarDefault: number;
  urutan: Jenis[];
};

export function indeks(peta: PetaIkon): Indeks {
  const nama = new Map<string, Aturan>();
  const awalan: Aturan[] = [];
  const akhiran: Aturan[] = [];
  const ext = new Map<string, Aturan>();
  const dir = new Map<string, Aturan>();
  for (const a of peta.aturan) {
    if (a.jenis === "nama") nama.set(a.kunci, a);
    else if (a.jenis === "awalan") awalan.push(a);
    else if (a.jenis === "akhiran") akhiran.push(a);
    else if (a.jenis === "ext") ext.set(a.kunci, a);
    else if (a.jenis === "dir") dir.set(a.kunci, a);
  }
  // Yang terpanjang dulu, supaya "readme" menang atas awalan yang lebih umum.
  const urut = (a: Aturan, b: Aturan) => b.kunci.length - a.kunci.length;
  awalan.sort(urut);
  akhiran.sort(urut);
  return { nama, awalan, akhiran, ext, dir, bawaan: peta.bawaan, lebarDefault: peta.lebar_default, urutan: peta.urutan };
}

function jadi(
  a: Aturan | undefined,
  bawaan: PetaIkon["bawaan"],
  jenis: Jenis,
  pola: string,
  lebarDefault: number,
): Hasil {
  // Tanpa aturan yang cocok, glyph dan warnanya datang dari kategori bawaan —
  // bukan karakter pengganti. Kalau glyph-nya juga tidak ada, baru `"?"`.
  const codepoint = a?.codepoint ?? bawaan.codepoint;
  return {
    ch: codepoint != null ? String.fromCodePoint(codepoint) : "?",
    fallback: a?.fallback ?? bawaan.fallback,
    lebar: a?.lebar ?? lebarDefault,
    keluarga: a?.keluarga ?? bawaan.keluarga,
    warna: a?.warna ?? bawaan.warna,
    jenis,
    pola,
  };
}

/** Ikon untuk nama folder. */
export function ikonFolder(ix: Indeks, nama: string): Hasil {
  const key = nama.toLowerCase();
  const a = ix.dir.get(key);
  return jadi(a, ix.bawaan.folder, a ? "dir" : "bawaan", a ? key : "", ix.lebarDefault);
}

/** Ikon untuk nama berkas, memakai urutan dari data. */
export function ikonBerkas(ix: Indeks, nama: string): Hasil {
  const key = nama.toLowerCase();
  for (const tahap of ix.urutan) {
    switch (tahap) {
      case "nama": {
        const a = ix.nama.get(key);
        if (a) return jadi(a, ix.bawaan.berkas, "nama", key, ix.lebarDefault);
        break;
      }
      case "akhiran": {
        const a = ix.akhiran.find((r) => key.endsWith(r.kunci));
        if (a) return jadi(a, ix.bawaan.berkas, "akhiran", a.kunci, ix.lebarDefault);
        break;
      }
      case "awalan": {
        const a = ix.awalan.find((r) => key.startsWith(r.kunci));
        if (a) return jadi(a, ix.bawaan.berkas, "awalan", a.kunci, ix.lebarDefault);
        break;
      }
      case "ext": {
        // Dari titik yang paling kiri: ".tar.gz" menang atas ".gz".
        for (let i = key.indexOf("."); i >= 0; i = key.indexOf(".", i + 1)) {
          const a = ix.ext.get(key.slice(i));
          if (a) return jadi(a, ix.bawaan.berkas, "ext", key.slice(i), ix.lebarDefault);
        }
        break;
      }
      case "bawaan": {
        return jadi(undefined, ix.bawaan.berkas, "bawaan", "", ix.lebarDefault);
      }
    }
  }
  return jadi(undefined, ix.bawaan.berkas, "bawaan", "", ix.lebarDefault);
}
