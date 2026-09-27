//! Icon pack: satu berkas yang **menimpa sebagian** peta ikon, bukan
//! menggantinya seluruhnya.
//!
//! # Kenapa perlu, dan kenapa `--icons-map` saja tidak cukup
//!
//! `--icons-map` memuat peta `ticon-map/2` yang **lengkap**: `families`,
//! `defaults`, dan seluruh aturan. Itu benar untuk "saya mau ikon saya sendiri",
//! tapi salah untuk kasus yang paling sering muncul — "pakai set ikon ini, atau
//! ganti 5 ekstensi saja". Tanpa pewarisan, pemakainya harus menyalin ulang
//! peta 150 KiB, dan salinan itu cepat basi.
//!
//! Pack menutupnya: isinya **sebagian**, dan setiap bagian digabung ke peta
//! dasar. Aturan dengan pasangan (tahap, kunci) yang sudah ada **diganti**, bukan
//! ditumpuk — jadi "ganti 5 ekstensi" benar-benar hanya lima baris.
//!
//! # Bentuk cadangan sebagai data, bukan enum di Rust
//!
//! Mode tanpa Nerd Font butuh tabel karakter per keluarga warna. Kalau tabel itu
//! hidup di dalam kode Rust, ia ikut hilang saat refactor dan tidak bisa
//! diaudit dari luar. Di pack, ia jadi data: bisa diaudit `tools/audit-font.py`,
//! bisa diganti pengguna, dan bisaPert Saying metadata-nya.
//!
//! # Kontrak
//!
//! JSON, versi `ticon-pack/1`. Semua bagian opsional kecuali `schema` dan
//! `pack`:
//!
//! ```json
//! {
//!   "schema": "ticon-pack/1",
//!   "pack": { "name": "contoh", "version": "1.0.0", "description": "..." },
//!   "families": { "khas": "magenta" },
//!   "rules": [
//!     { "kind": "ext", "key": ".log", "family": "khas", "glyph": "nf-md-file_outline" }
//!   ],
//!   "shapes": { "khas": "*" },
//!   "folder_shape": "|"
//! }
//! ```
//!
//! `families` dibaca lebih dulu, jadi `rules` dan `shapes` boleh menyebut keluarga
//! yang baru ditambahkan pack yang sama.
//!
//! Yang **ditolak**, bukan diabaikan diam-diam: kunci yang tidak dikenal di
//! semua tingkat, kunci kosong, karakter kontrol/bidi di setiap string (lewat
//! gerbang yang sama dengan peta netral), keluarga yang tidak ada, warna yang
//! tidak ada di palet, glyph yang tidak dikenal, nilai `shapes` yang bukan tepat
//! satu karakter, `min_ticon` yang lebih baru dari `ticon` ini, dan dua aturan
//! dengan (tahap, kunci) sama di dalam satu pack — yang terakhir ditolak karena
//! "mana yang menang?" tidak punya jawaban yang bisa ditebak.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::json::{self, Nilai};
use crate::mapping::{Category, DirRule, Rules};

/// Id skema yang dibaca versi ini.
pub const SKEMA: &str = "ticon-pack/1";

/// Nama pack bawaan: bentuk satu sel per keluarga warna, tanpa Nerd Font.
pub const NAMA_BAWAAN: &str = "shape";

/// Nama pack bawaan kedua: bentuk yang **pasti** satu sel di semua terminal.
///
/// Bedanya dengan [`NAMA_BAWAAN`] ada di kolom `East_Asian_Width`. Karakter di
/// sini hanya `N`/`Na`; pack [`NAMA_BAWAAN`] memakai karakter `A`
/// (*ambiguous*) yang menjadi dua sel di terminal yang dikonfigurasi
/// *ambiguous = lebar* — termasuk yang sudah punya Nerd Font terpasang.
/// `tools/audit-font.py --sempit` yang mengukurnya, bukan majas datas.
pub const NAMA_SEMPIT: &str = "sempit";

const BAWAAN_JSON: &str = include_str!("../packs/shape/pack.json");
const SEMPIT_JSON: &str = include_str!("../packs/sempit/pack.json");

/// Kunci yang boleh di level atas. `$schema` dikecualikan seperti di peta netral.
const KUNCI_TOP: [&str; 7] = [
    "$schema",
    "schema",
    "pack",
    "families",
    "rules",
    "shapes",
    "folder_shape",
];

const KUNCI_META: [&str; 6] = [
    "name",
    "version",
    "description",
    "author",
    "license",
    "min_ticon",
];

/// `fallback` sengaja tidak ada di sini, meski peta netral punya: pemuat pack
/// tidak bisa mengaplikasikannya, jadi menerimanya berarti diam-diam mengabaikan
/// field yang ditulis pengguna. Menolaknya lebih jujur.
const KUNCI_ATURAN: [&str; 4] = ["kind", "key", "family", "glyph"];

/// Batas ukuran berkas pack: sama dengan peta netral. Pack tulis-tangan adalah
/// berkas kecil, dan tanpa batas `--icons-pack` bisa ditunjuk ke apa saja.
pub const MAKS_BERKAS: u64 = crate::peta::MAKS_BERKAS;

/// Tahap pencocokan aturan pack. Sama dengan [`crate::mapping::Prioritas`],
/// ditambah `folder` yang tidak punya tahap padanannya di sana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tahap {
    /// Nama berkas persis.
    Nama,
    /// Akhiran nama.
    Akhiran,
    /// Awalan nama.
    Awalan,
    /// Ekstensi.
    Ekstensi,
    /// Nama folder well-known.
    Folder,
}

impl Tahap {
    /// Nilai `kind` di kontrak pack. Huruf besar-kecil, tidak pernah berubah.
    pub fn kind(self) -> &'static str {
        match self {
            Tahap::Nama => "name",
            Tahap::Akhiran => "suffix",
            Tahap::Awalan => "prefix",
            Tahap::Ekstensi => "ext",
            Tahap::Folder => "dir",
        }
    }

    /// Kebalikan dari [`Tahap::kind`].
    pub fn dari_kind(kind: &str) -> Option<Self> {
        match kind {
            "name" => Some(Tahap::Nama),
            "suffix" => Some(Tahap::Akhiran),
            "prefix" => Some(Tahap::Awalan),
            "ext" => Some(Tahap::Ekstensi),
            "dir" => Some(Tahap::Folder),
            _ => None,
        }
    }
}

/// Metadata pack: siapa yang membuatnya, dan versi `ticon` yang dibutuhkannya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    /// Nama pack. Harus sama dengan nama yang dicari kalau asalnya bawaan —
    /// kalau tidak, muncul pertanyaan "kenapa `--icons-pack nord` memakai glyph
    /// pack `shape`?" yang tidak ada jawabannya.
    pub nama: String,
    /// Versi pack, bebas format (`1.0.0`, `2026-09-27`, ...).
    pub versi: String,
    /// Keterangan singkat; ditampilkan `--icons-list-packs`.
    pub deskripsi: Option<String>,
    /// Siapa yang menulis pack.
    pub penulis: Option<String>,
    /// Lisensi pack. Dicatat, tidak ditegakkan: pemakai yang menilai.
    pub lisensi: Option<String>,
    /// Versi `ticon` paling tua yang bisa memakainya.
    pub min_ticon: Option<String>,
}

/// Dari mana pack dimuat. Beda antara "bawaan `ticon`" dan "berkas di disk"
/// penting: yang bawaan ikut ter-*package*, jadi tidak bisa hilang diam-diam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asal {
    /// Pack tertanam di biner.
    Bawaan(&'static str),
    /// Berkas di disk.
    Berkas(PathBuf),
}

impl Asal {
    /// Label satu baris untuk daftar pack.
    pub fn label(&self) -> String {
        match self {
            Asal::Bawaan(nama) => format!("bawaan ({nama})"),
            Asal::Berkas(path) => path.display().to_string(),
        }
    }
}

/// Bentuk cadangan per keluarga warna, untuk dipakai ketika glyph Nerd Font
/// tidak bisa digambar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bentuk {
    /// Nama keluarga -> satu karakter.
    pub keluarga: BTreeMap<String, char>,
    /// Bentuk untuk folder. Kalau kosong, folder memakai bentuk keluarganya.
    pub folder: Option<char>,
}

impl Bentuk {
    /// Bentuk kosong: pakai glyph Nerd Font seperti biasa.
    pub fn kosong() -> Self {
        Self::default()
    }

    /// True kalau pack ini tidak menyediakan bentuk cadangan sama sekali.
    pub fn is_kosong(&self) -> bool {
        self.keluarga.is_empty() && self.folder.is_none()
    }

    /// Bentuk untuk sebuah ikon.
    ///
    /// Dicari lewat **keluarga**, bukan lewat nama warna: `Rules` menegakkan satu
    /// warna = satu keluarga, jadi [`Rules::family_dari_warna`] juga tunggal.
    /// Kalau keluarga tidak punya bentuk, hasilnya `None` — bukan karakter
    /// sembarang — supaya pemanggil tahu glyph aslinya tidak tergantikan.
    pub fn untuk(&self, rules: &Rules, color: &str, folder: bool) -> Option<char> {
        if folder {
            if let Some(bentuk) = self.folder {
                return Some(bentuk);
            }
        }
        let keluarga = rules.family_dari_warna(color)?;
        self.keluarga.get(keluarga).copied()
    }

    /// Semua karakter yang dipakai pack ini, untuk diaudit dari luar.
    pub fn semua_karakter(&self) -> Vec<char> {
        let mut semua: Vec<char> = self.keluarga.values().copied().collect();
        if let Some(bentuk) = self.folder {
            semua.push(bentuk);
        }
        semua.sort_unstable_by_key(|c| *c as u32);
        semua.dedup();
        semua
    }
}

/// Pack yang sudah dimuat: metadata, peta hasil penggabungan, dan bentuk cadangan.
///
/// #[derive(Debug)] sengaja tidak ada: Rules tidak mengimplementasinya, dan
///// menurunkannya cuma untuk facility yang tidak dipakai akan jadi hutang saat
///// Rules berubah.
pub struct Pack {
    /// Metadata pack.
    pub meta: Meta,
    /// Dari mana asalnya.
    pub asal: Asal,
    /// Peta ikon setelah pack digabung ke peta dasar.
    pub rules: Rules,
    /// Bentuk cadangan, kalau pack menyediakannya.
    pub bentuk: Bentuk,
}

/// Satu baris `--icons-list-packs`.
#[derive(Debug, Clone)]
pub struct Info {
    /// Nama pack.
    pub nama: String,
    /// Asalnya.
    pub asal: Asal,
    /// Metadata; `Err` kalau pack rusak. Kesalahannya ikut ditampilkan — pack
    /// yang ada di disk tapi tidak termuat adalah informasi, bukan rahasia.
    pub meta: Result<Meta, String>,
}

/// Satu aturan dari pack, sudah divalidasi tapi belum digabung.
#[derive(Debug, Clone)]
struct Aturan {
    tahap: Tahap,
    kunci: String,
    glyph: String,
    family: String,
}

/// Teks pack bawaan, kalau `nama` cocok dengan salah satunya.
pub fn bawaan(nama: &str) -> Option<(&'static str, &'static str)> {
    match nama {
        NAMA_BAWAAN => Some((NAMA_BAWAAN, BAWAAN_JSON)),
        NAMA_SEMPIT => Some((NAMA_SEMPIT, SEMPIT_JSON)),
        _ => None,
    }
}

/// Muat pack dari teks JSON, lalu gabungkan ke peta dasar.
///
/// `dasar` hanya dibaca; peta hasil dikembalikan di [`Pack::rules`], jadi
/// pemanggil yang memutuskan memakainya atau tidak.
pub fn muat_teks(teks: &str, asal: Asal, dasar: &Rules) -> Result<Pack, String> {
    let teks = teks.strip_prefix('\u{feff}').unwrap_or(teks);
    let akar = json::urai(teks).map_err(|e| format!("icon pack tidak bisa dibaca: {e}"))?;
    cek_kunci_top(&akar)?;

    let skema = akar.ambil("schema").and_then(Nilai::teks).unwrap_or("");
    if skema != SKEMA {
        return Err(format!(
            "skema harus \"{SKEMA}\", ditemukan \"{}\"",
            crate::render::sanitize(skema)
        ));
    }

    let meta = Meta::dari_node(&akar)?;
    cek_min_ticon(&meta)?;
    if let Asal::Bawaan(nama) = &asal {
        if meta.nama != *nama {
            return Err(format!(
                "pack bawaan `{nama}` tetapi `pack.name` = \"{}\"",
                crate::render::sanitize(&meta.nama)
            ));
        }
    }

    // Keluarga lebih dulu, supaya `rules` dan `shapes` boleh memakai keluarga
    // yang baru ditambahkan pack ini.
    let mut families = dasar.families.clone();
    if let Some(node) = akar.ambil("families") {
        for (nama, nilai) in objek(node, "families")? {
            let posisi = format!("families.{nama}");
            let warna = wajib_teks(nilai, &posisi)?;
            if !dasar.palette.contains_key(&warna) {
                return Err(format!(
                    "{posisi}: warna \"{}\" tidak ada di palet ({})",
                    crate::render::sanitize(&warna),
                    palet_daftar(dasar)
                ));
            }
            families.insert(nama.clone(), warna);
        }
    }

    let mut categories = dasar.categories.clone();
    let mut dirs = dasar.dirs.clone();
    let mut diklaim: Vec<(Tahap, String)> = Vec::new();
    if let Some(node) = akar.ambil("rules") {
        for (i, nilai) in node.array().unwrap_or_default().iter().enumerate() {
            let posisi = format!("rules[{i}]");
            let aturan = Aturan::dari_node(nilai, &posisi, &families)?;
            if diklaim.contains(&(aturan.tahap, aturan.kunci.clone())) {
                return Err(format!(
                    "{posisi}: aturan `{}` untuk \"{}\" diulang dalam pack yang sama; \
                     dua aturan kembar tidak punya pemenang yang bisa ditebak",
                    aturan.tahap.kind(),
                    crate::render::sanitize(&aturan.kunci)
                ));
            }
            diklaim.push((aturan.tahap, aturan.kunci.clone()));
            terapkan(&mut categories, &mut dirs, &aturan);
        }
    }

    let rules = Rules::dari_bagian(dasar.palette.clone(), families, categories, dirs)?;
    let bentuk = bentuk_dari_node(&akar, &rules)?;

    Ok(Pack {
        meta,
        asal,
        rules,
        bentuk,
    })
}

/// Muat pack dari berkas, dengan batas ukuran yang sama seperti peta netral.
pub fn muat_berkas(path: &Path, dasar: &Rules) -> Result<Pack, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{}: bukan berkas biasa", path.display()));
    }
    if meta.len() > MAKS_BERKAS {
        return Err(format!(
            "{}: pack terlalu besar ({} KiB; batas {} KiB)",
            path.display(),
            meta.len() / 1024,
            MAKS_BERKAS / 1024
        ));
    }
    let teks = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    muat_teks(&teks, Asal::Berkas(path.to_path_buf()), dasar)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Muat pack berdasarkan **nama**.
///
/// Nama yang memuat pemisah path atau berakhiran `.json` diperlakukan sebagai
/// path, jadi `ticon --icons-pack ./packs/nord/pack.json` tetap jalan tanpa opsi
/// terpisah. Selain itu nama dicari di lokasi yang didokumentasikan
/// (lihat [`lokasi`]), dengan pack bawaan menang lebih dulu supaya
/// `--icons-pack shape` selalu berarti yang tertanam, bukan berkas acak di cwd.
pub fn muat_nama(nama: &str, dasar: &Rules) -> Result<Pack, String> {
    if nama.is_empty() {
        return Err("nama pack kosong".to_string());
    }
    if let Some((bawaan, json_teks)) = bawaan(nama) {
        return muat_teks(json_teks, Asal::Bawaan(bawaan), dasar);
    }
    if nama.contains('/') || nama.contains('\\') || nama.ends_with(".json") {
        return muat_berkas(Path::new(nama), dasar);
    }

    let kandidat = lokasi(nama);
    for path in &kandidat {
        if path.is_file() {
            return muat_berkas(path, dasar);
        }
    }
    Err(format!(
        "pack \"{}\" tidak ditemukan. Dicari di:\n  {}\n\
         `ticon --icons-list-packs` menampilkan yang tersedia; \
         path langsung juga bisa: `--icons-pack ./nama/pack.json`",
        crate::render::sanitize(nama),
        kandidat
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    ))
}

/// Direktori tempat pack dicari, urut dari yang paling khusus.
///
/// `TICON_PACKS` lebih dulu supaya uji dan skrip bisa menunjuk direktori sendiri
/// tanpa mengubah apa pun di disk.
fn akar_pencarian() -> Vec<PathBuf> {
    let mut akar: Vec<PathBuf> = Vec::new();
    if let Ok(daftar) = std::env::var("TICON_PACKS") {
        let pisah = if cfg!(windows) { ';' } else { ':' };
        akar.extend(
            daftar
                .split(pisah)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
    }
    if let Some(profile) = std::env::var_os("LOCALAPPDATA") {
        akar.push(Path::new(&profile).join("ticon-packs"));
    }
    if let Some(rumah) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let rumah = PathBuf::from(rumah);
        akar.push(
            rumah
                .join(".local")
                .join("share")
                .join("ticon")
                .join("packs"),
        );
        akar.push(rumah.join(".config").join("ticon").join("packs"));
    }
    akar
}

/// Semua lokasi pen-pack `nama`, berurutan dari yang paling khusus.
///
/// Urutannya bagian dari kontrak, jadi tidak boleh diubah diam-diam:
/// 1. `$TICON_PACKS` — daftar direktori, dipisah `;` (Windows) atau `:` (Unix).
/// 2. `./ticon-packs/<nama>/pack.json` — per-proyek, ikut kontrol versi.
/// 3. `%LOCALAPPDATA%\ticon-packs\<nama>\pack.json` (Windows).
/// 4. `~/.local/share/ticon/packs/<nama>/pack.json` (XDG).
/// 5. `~/.config/ticon/packs/<nama>/pack.json`.
pub fn lokasi(nama: &str) -> Vec<PathBuf> {
    let mut keluar: Vec<PathBuf> = vec![PathBuf::from("ticon-packs").join(nama).join("pack.json")];
    for dir in akar_pencarian() {
        let candidate = dir.join(nama).join("pack.json");
        if !keluar.contains(&candidate) {
            keluar.push(candidate);
        }
    }
    keluar
}

/// Semua pack yang bisa ditemukan: bawaan dulu, lalu isi direktori pen-pack.
pub fn daftar() -> Vec<Info> {
    let mut keluar: Vec<Info> = vec![
        info_dari_teks(NAMA_BAWAAN, NAMA_BAWAAN, BAWAAN_JSON),
        info_dari_teks(NAMA_SEMPIT, NAMA_SEMPIT, SEMPIT_JSON),
    ];
    let mut akar = vec![PathBuf::from("ticon-packs")];
    akar.extend(akar_pencarian());

    for dir in akar {
        let Ok(isi) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entri in isi.flatten() {
            let candidate = entri.path().join("pack.json");
            if !candidate.is_file() {
                continue;
            }
            let Some(nama) = entri.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if keluar.iter().any(|i| i.nama == nama) {
                continue;
            }
            let meta = std::fs::read_to_string(&candidate)
                .map_err(|e| e.to_string())
                .and_then(|teks| {
                    json::urai(&teks)
                        .map_err(|e| format!("tidak bisa dibaca: {e}"))
                        .and_then(|akar| {
                            cek_kunci_top(&akar)?;
                            Meta::dari_node(&akar)
                        })
                });
            keluar.push(Info {
                nama,
                asal: Asal::Berkas(candidate),
                meta,
            });
        }
    }
    keluar
}

fn info_dari_teks(nama: &str, asal: &'static str, json_teks: &str) -> Info {
    let meta = json::urai(json_teks)
        .map_err(|e| e.to_string())
        .and_then(|akar| {
            cek_kunci_top(&akar)?;
            Meta::dari_node(&akar)
        });
    Info {
        nama: nama.to_string(),
        asal: Asal::Bawaan(asal),
        meta,
    }
}

fn palet_daftar(rules: &Rules) -> String {
    rules.palette.keys().cloned().collect::<Vec<_>>().join(", ")
}

fn cek_kunci_top(akar: &Nilai) -> Result<(), String> {
    let Nilai::Object(peta) = akar else {
        return Err("akar pack harus objek JSON".to_string());
    };
    for kunci in peta.keys() {
        if kunci == "$schema" || KUNCI_TOP.contains(&kunci.as_str()) {
            continue;
        }
        return Err(format!(
            "kunci level atas \"{kunci}\" tidak dikenal oleh {SKEMA} (yang dikenal: {})",
            KUNCI_TOP.join(", ")
        ));
    }
    Ok(())
}

impl Meta {
    fn dari_node(akar: &Nilai) -> Result<Self, String> {
        let node = akar
            .ambil("pack")
            .ok_or_else(|| "`pack` wajib ada (metadata pack)".to_string())?;
        if !matches!(node, Nilai::Object(_)) {
            return Err("`pack` harus objek".to_string());
        }
        for kunci in kunci_objek(node) {
            if !KUNCI_META.contains(&kunci.as_str()) {
                return Err(format!(
                    "pack.{kunci} tidak dikenal oleh {SKEMA} (yang dikenal: {})",
                    KUNCI_META.join(", ")
                ));
            }
        }
        Ok(Self {
            nama: wajib_kunci(node, "name", "pack")?,
            versi: wajib_kunci(node, "version", "pack")?,
            deskripsi: opsional_teks(node, "description")?,
            penulis: opsional_teks(node, "author")?,
            lisensi: opsional_teks(node, "license")?,
            min_ticon: opsional_teks(node, "min_ticon")?,
        })
    }
}

/// Bandingkan `min_ticon` dengan versi `ticon` yang sedang berjalan. Hanya dua
/// komponen pertama yang dibandingkan — `0.4.3-beta` dianggap `0.4.3`, dan
/// prerelease memang tidak pernah lebih baru daripada rilisnya.
fn cek_min_ticon(meta: &Meta) -> Result<(), String> {
    let Some(min) = &meta.min_ticon else {
        return Ok(());
    };
    let ambil = |teks: &str| -> Vec<u32> {
        teks.split(['.', '-', '+'])
            .take(2)
            .filter_map(|bagian| bagian.parse::<u32>().ok())
            .collect()
    };
    if ambil(min) > ambil(env!("CARGO_PKG_VERSION")) {
        return Err(format!(
            "pack \"{}\" {} butuh ticon {min} atau lebih baru; yang jalan ini {}",
            crate::render::sanitize(&meta.nama),
            crate::render::sanitize(&meta.versi),
            env!("CARGO_PKG_VERSION")
        ));
    }
    Ok(())
}

impl Aturan {
    fn dari_node(
        nilai: &Nilai,
        posisi: &str,
        families: &BTreeMap<String, String>,
    ) -> Result<Self, String> {
        for kunci in kunci_objek(nilai) {
            if !KUNCI_ATURAN.contains(&kunci.as_str()) {
                return Err(format!(
                    "{posisi}: kunci \"{kunci}\" tidak dikenal oleh {SKEMA} \
                     (yang dikenal: {})",
                    KUNCI_ATURAN.join(", ")
                ));
            }
        }
        let kind = wajib_kunci(nilai, "kind", posisi)?;
        let tahap = Tahap::dari_kind(&kind)
            .ok_or_else(|| format!("{posisi}: kind \"{kind}\" tidak dikenal"))?;
        let kunci = wajib_kunci(nilai, "key", posisi)?;
        cek_kunci_aturan(tahap, &kunci, posisi)?;
        let family = wajib_kunci(nilai, "family", posisi)?;
        if !families.contains_key(&family) {
            return Err(format!(
                "{posisi}: keluarga \"{family}\" tidak ada (yang ada: {})",
                families.keys().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        let glyph = wajib_kunci(nilai, "glyph", posisi)?;
        if crate::glyph::Glyphs::bundled().get(&glyph).is_none() {
            return Err(format!("{posisi}: glyph \"{glyph}\" tidak dikenal"));
        }
        Ok(Self {
            tahap,
            kunci,
            glyph,
            family,
        })
    }
}

/// Validasi kunci aturan: aturan yang tidak akan pernah menang ditolak di
/// tempatnya, bukan diam-diam jadi tidak berlaku.
fn cek_kunci_aturan(tahap: Tahap, kunci: &str, posisi: &str) -> Result<(), String> {
    let bersih = crate::render::sanitize(kunci);
    match tahap {
        Tahap::Ekstensi if !kunci.starts_with('.') => Err(format!(
            "{posisi}: ekstensi \"{bersih}\" harus diawali titik"
        )),
        Tahap::Folder if kunci != kunci.to_lowercase() => Err(format!(
            "{posisi}: nama folder \"{bersih}\" harus huruf kecil"
        )),
        _ => Ok(()),
    }
}

/// Gabungkan satu aturan ke peta: cabut kuncinya dari mana pun, lalu masukkan ke
/// kategori (keluarga, glyph) yang diminta.
///
/// Mematikan kunci dari **semua** kategori lebih dulu adalah inti semantik
/// "ganti": kalau tidak, aturan lama dan baru bisa hidup berdampingan dan
/// urutan pencocokan yang menentukan pemenang — persis hal yang tidak boleh
/// terjadi diam-diam.
fn terapkan(
    categories: &mut BTreeMap<String, Category>,
    dirs: &mut BTreeMap<String, DirRule>,
    aturan: &Aturan,
) {
    for kategori in categories.values_mut() {
        kategori.names.retain(|k| k != &aturan.kunci);
        kategori.suffix.retain(|k| k != &aturan.kunci);
        kategori.prefix.retain(|k| k != &aturan.kunci);
        kategori.ext.retain(|k| k != &aturan.kunci);
        // `by_ext` ikut dicabut: kalau tidak, entri basi bertahan dan glyph
        // lama bisa muncul lagi kalau kuncinya suatu saat ditambahkan balik.
        kategori.by_ext.remove(&aturan.kunci);
    }
    dirs.remove(&aturan.kunci.to_lowercase());

    if aturan.tahap == Tahap::Folder {
        dirs.insert(
            aturan.kunci.to_lowercase(),
            DirRule {
                glyph: aturan.glyph.clone(),
                family: aturan.family.clone(),
                lebar: None,
                fallback: None,
            },
        );
        return;
    }

    let nama = nama_kategori(categories, &aturan.family, &aturan.glyph);
    let kategori = categories.entry(nama).or_insert_with(|| Category {
        glyph: aturan.glyph.clone(),
        family: aturan.family.clone(),
        ext: Vec::new(),
        names: Vec::new(),
        prefix: Vec::new(),
        suffix: Vec::new(),
        by_ext: BTreeMap::new(),
        lebar: None,
        fallback: None,
    });
    let kunci = aturan.kunci.clone();
    match aturan.tahap {
        Tahap::Nama => kategori.names.push(kunci),
        Tahap::Akhiran => kategori.suffix.push(kunci),
        Tahap::Awalan => kategori.prefix.push(kunci),
        Tahap::Ekstensi => kategori.ext.push(kunci),
        Tahap::Folder => unreachable!("folder ditangani di atas"),
    }
}

/// Nama kategori untuk sepasang (keluarga, glyph): pakai nama yang sudah ada kalau
/// pasangannya cocok, kalau tidak buat yang deterministik.
fn nama_kategori(categories: &BTreeMap<String, Category>, family: &str, glyph: &str) -> String {
    for (nama, kategori) in categories {
        if kategori.family == family && kategori.glyph == glyph {
            return nama.clone();
        }
    }
    format!("{family}:{glyph}")
}

/// Baca `shapes` dan `folder_shape` dari pack.
///
/// `shapes` diperiksa **setelah** peta digabung, jadi pack boleh memakai keluarga
/// yang baru ia tambahkan sendiri di `families`.
fn bentuk_dari_node(akar: &Nilai, rules: &Rules) -> Result<Bentuk, String> {
    let mut bentuk = Bentuk::kosong();
    if let Some(node) = akar.ambil("shapes") {
        for (keluarga, nilai) in objek(node, "shapes")? {
            let posisi = format!("shapes.{keluarga}");
            if !rules.families.contains_key(keluarga.as_str()) {
                return Err(format!(
                    "{posisi}: keluarga \"{keluarga}\" tidak ada (yang ada: {})",
                    rules
                        .families
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            let teks = wajib_teks(nilai, &posisi)?;
            bentuk
                .keluarga
                .insert(keluarga.clone(), satu_karakter(&teks, &posisi)?);
        }
    }
    if let Some(node) = akar.ambil("folder_shape") {
        let teks = node
            .teks()
            .ok_or_else(|| "`folder_shape` harus berupa teks".to_string())?;
        bentuk.folder = Some(satu_karakter(teks, "folder_shape")?);
    }
    Ok(bentuk)
}

/// `shapes` dan `folder_shape` harus **tepat satu** karakter: dua karakter
/// membuat janji "satu sel" jadi bohong dan merusak kolom.
fn satu_karakter(teks: &str, posisi: &str) -> Result<char, String> {
    let mut karakter = teks.chars();
    let Some(pertama) = karakter.next() else {
        return Err(format!("{posisi}: tidak boleh kosong"));
    };
    if karakter.next().is_some() {
        return Err(format!(
            "{posisi}: harus tepat satu karakter, ditemukan {}",
            crate::render::sanitize(teks)
        ));
    }
    cek_aman(posisi, teks)?;
    Ok(pertama)
}

fn objek<'a>(node: &'a Nilai, posisi: &str) -> Result<Vec<(String, &'a Nilai)>, String> {
    let Nilai::Object(peta) = node else {
        return Err(format!("`{posisi}` harus objek"));
    };
    Ok(peta
        .iter()
        .map(|(kunci, nilai)| (kunci.clone(), nilai))
        .collect())
}

fn kunci_objek(node: &Nilai) -> Vec<String> {
    match node {
        Nilai::Object(peta) => peta.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// Gerbang karakter kontrol/bidi, sama seperti peta netral: metadata pack ikut
/// dicetak ke terminal (`--icons-list-packs`), jadi isinya bukan data
/// tepercaya.
fn cek_aman(posisi: &str, nilai: &str) -> Result<(), String> {
    if crate::render::is_terminal_safe(nilai) {
        return Ok(());
    }
    Err(format!(
        "{} memuat karakter kontrol/bidi: {}",
        crate::render::sanitize(posisi),
        crate::render::sanitize(nilai)
    ))
}

/// Teks yang **nilai node-nya sudah** teks (mis. isi `shapes.kode`).
fn wajib_teks(nilai: &Nilai, posisi: &str) -> Result<String, String> {
    let teks = nilai
        .teks()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| format!("{posisi}: teks wajib ada dan tidak boleh kosong"))?;
    cek_aman(posisi, teks)?;
    Ok(teks.to_string())
}

/// Teks dari field objek, lewat gerbang yang sama. Dipakai untuk metadata pack,
/// yang datang sebagai `{"name": "..."}` — bukan sebagai nilai yang sudah teks.
fn wajib_kunci(node: &Nilai, kunci: &str, posisi: &str) -> Result<String, String> {
    let Some(isi) = node.ambil(kunci) else {
        return Err(format!("{posisi}.{kunci} wajib ada"));
    };
    wajib_teks(isi, &format!("{posisi}.{kunci}"))
}

fn opsional_teks(node: &Nilai, kunci: &str) -> Result<Option<String>, String> {
    let Some(isi) = node.ambil(kunci) else {
        return Ok(None);
    };
    if matches!(isi, Nilai::Null) {
        return Ok(None);
    }
    let teks = isi
        .teks()
        .ok_or_else(|| format!("{kunci} harus berupa teks"))?;
    cek_aman(kunci, teks)?;
    Ok(Some(teks.to_string()))
}
