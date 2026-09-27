//! Aturan pemetaan: memuat `icons.toml`, membangun indeks, dan menjawab
//! "berkas ini pakai ikon apa, warna apa".
//!
//! `icons.toml` adalah satu-satunya sumber kebenaran. Tidak ada tabel kembar
//! yang harus dijaga sinkron, jadi mustahil menambah ikon tanpa menambah
//! warnanya.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::glyph::Glyphs;
use crate::render;

/// Keluarga glyph yang boleh dipakai. Alasannya ada di `icons.toml` dan
/// `build.rs`: konsistensi visual.
pub const ALLOWED_FAMILIES: &[&str] = &["nf-md-", "nf-oct-"];

/// Kategori bawaan untuk folder yang namanya tidak dikenal. Wajib ada di
/// `icons.toml` — bahkan nilai bawaan pun data, bukan hardcode di kode.
pub const DEFAULT_DIR_CATEGORY: &str = "folder";
/// Kategori bawaan untuk berkas yang tidak cocok aturan apa pun.
pub const DEFAULT_FILE_CATEGORY: &str = "file";

/// Teks `icons.toml` yang sama persis dengan yang dimuat `Rules::load()`.
/// Sengaja dibaca sebagai teks: urutan editorial — yang dijaga oleh tes
/// ketetanggaan — tidak hidup di struktur data mana pun, sebab `categories`
/// berupa BTreeMap dan urutannya hilang begitu di-parse.
pub const ICONS_TOML: &str = include_str!("../icons.toml");

/// Batas pemakaian warna `dim`, dalam satuan aturan (kategori + folder).
/// `dim` berarti "tidak ada yang menarik di sini": berkas tak dikenal dan
/// artefak hasil generate. Dulu ia menutupi 21 aturan dan membuat daftar
/// terasa mati; sekarang 9, dan angka inilah yang menjaga ia tidak merayap
/// naik lagi — menaikkannya harus lewat keputusan sadar di sini.
pub const MAX_DIM_RULES: usize = 10;

#[derive(Debug, Clone, Deserialize)]
/// Satu blok `[categories.<nama>]` di `icons.toml`: bentuk ikon dan cakupannya.
/// Tidak ada warna di sini — hanya keluarga, dan warnanya diturunkan.
pub struct Category {
    /// Nama glyph (harus dari keluarga yang diizinkan `build.rs`).
    pub glyph: String,
    /// Keluarga dari tabel `[families]`. Warnanya diturunkan lewat
    /// `Rules::color_of`, jadi keputusan warna hanya hidup di satu tempat.
    pub family: String,
    /// Ekstensi (huruf kecil, diawali titik). `.tar.gz` menang atas `.gz`.
    #[serde(default)]
    pub ext: Vec<String>,
    /// Nama berkas persis, tanpa wildcard.
    #[serde(default)]
    pub names: Vec<String>,
    /// Awalan nama berkas.
    #[serde(default)]
    pub prefix: Vec<String>,
    /// Akhiran nama berkas.
    #[serde(default)]
    pub suffix: Vec<String>,
    /// Bentuk berbeda di dalam kategori yang sama. Warna tetap milik kategori,
    /// jadi variasi bentuk tidak menambah warna baru.
    #[serde(default)]
    pub by_ext: BTreeMap<String, String>,
}

/// Aturan folder well-known dari bagian `[dirs]`: bentuk dan keluarga warna.
#[derive(Debug, Clone, Deserialize)]
pub struct DirRule {
    /// Nama glyph untuk folder ini.
    pub glyph: String,
    /// Keluarga warnanya; lihat [`Category::family`].
    pub family: String,
}

#[derive(Debug, Deserialize)]
struct IconsFile {
    palette: BTreeMap<String, String>,
    families: BTreeMap<String, String>,
    categories: BTreeMap<String, Category>,
    #[serde(default)]
    dirs: BTreeMap<String, DirRule>,
}

/// Dari mana sebuah aturan berasal. Urutan enum ini juga urutan prioritasnya,
/// dan urutan itulah yang dikunci tes `urutan_resolusi_ikut_dokumentasi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchedBy {
    /// Nama folder dari `[dirs]`.
    WellKnownFolder,
    /// Nama berkas persis dari `names`.
    Name,
    /// Akhiran dari `suffix`.
    Suffix,
    /// Awalan dari `prefix`.
    Prefix,
    /// Ekstensi dari `ext`.
    Extension,
    /// Tidak ada aturan yang cocok; dipakai kategori bawaan.
    Fallback,
}

impl MatchedBy {
    /// Label siap tampil, untuk `ikon --explain` dan dokumentasi.
    pub fn label(self) -> &'static str {
        match self {
            MatchedBy::WellKnownFolder => "folder well-known",
            MatchedBy::Name => "nama persis",
            MatchedBy::Suffix => "akhiran",
            MatchedBy::Prefix => "awalan",
            MatchedBy::Extension => "ekstensi",
            MatchedBy::Fallback => "kategori bawaan",
        }
    }
}

/// Hasil resolusi: nama glyph + nama warna dari palet. Keduanya masih berupa
/// nama, bukan karakter dan kode ANSI — indirection inilah yang membuat
/// resolve dan render bisa berubah sendiri-sendiri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// Nama glyph, bukan karakter — [`crate::glyph::Glyphs`] yang menerjemahkannya.
    pub glyph: String,
    /// Nama warna dari palet, bukan kode ANSI.
    pub color: String,
    /// Aturan mana yang menang. Penting untuk TUI: aplikasi bisa menampilkan
    /// "kenapa berkas ini jadi ikon ini" tanpa menebak.
    pub matched_by: MatchedBy,
}

impl Resolved {
    fn new(glyph: impl Into<String>, color: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            color: color.into(),
            matched_by: MatchedBy::Fallback,
        }
    }
}

/// Bentuk siap pakai untuk TUI atau skrip: karakter, nama warna, dan asal
/// pencocokannya. Tanpa string ANSI — pemanggil yang memilih cara mewarnainya.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    /// Karakter glyph, atau `None` bila nama glyph tidak ada di
    /// `assets/glyphs.toml`. Tidak ditebak dengan karakter pengganti supaya
    /// pemanggil tahu ada data yang kurang.
    pub ch: Option<char>,
    /// Nama warna dari `[palette]`, bukan kode ANSI.
    pub color: String,
    /// Aturan mana yang menang.
    pub matched_by: MatchedBy,
}

/// Satu aturan yang cocok untuk sebuah nama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kandidat {
    /// Jenis aturan ini (nama persis, akhiran, awalan, dst).
    pub matched_by: MatchedBy,
    /// Pola yang cocok, persis seperti tertulis di `icons.toml`.
    pub pattern: String,
    /// Nama glyph yang akan dipakai kalau aturan ini menang.
    pub glyph: String,
    /// Nama warnanya.
    pub color: String,
}

/// Hasil [`Rules::explain_file`] dan [`Rules::explain_dir`]: semua aturan yang
/// cocok, **berurutan prioritas**. Yang pertama adalah pemenang; sisanya kalah
/// dan menjelaskan kenapa hasilnya berbeda dari yang biasa orang kira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// Kandidat berurutan prioritas; yang pertama adalah pemenang.
    pub kandidat: Vec<Kandidat>,
}

impl Explanation {
    /// Aturan yang menang, atau `None` kalau tidak ada aturan yang cocok —
    /// dalam kasus itu berlaku kategori bawaan.
    pub fn winner(&self) -> Option<&Kandidat> {
        self.kandidat.first()
    }
}

/// Seluruh aturan pemetaan, sudah terindeks dan siap menjawab pertanyaan
/// "nama ini dapat ikon apa". Dibangun sekali lewat [`Rules::load`], lalu
/// dipakai berulang — pencarian tidak menyentuh berkas.
pub struct Rules {
    /// Nama warna -> kode SGR ANSI, dari `[palette]`.
    pub palette: BTreeMap<String, String>,
    /// Keluarga → warna. Di sinilah keputusan "siapa berbagi warna" dibuat;
    /// aturan tidak pernah menyebut warna, hanya keluarganya.
    pub families: BTreeMap<String, String>,
    /// Kategori dari `[categories.*]`, kunci huruf kecil.
    pub categories: BTreeMap<String, Category>,
    /// Aturan folder well-known dari `[dirs]`, kunci huruf kecil.
    pub dirs: BTreeMap<String, DirRule>,
    by_name: BTreeMap<String, Resolved>,
    by_ext: BTreeMap<String, Resolved>,
    /// Diurutkan dari yang terpanjang supaya "readme" menang atas aturan
    /// awalan yang lebih pendek dan lebih umum.
    prefixes: Vec<(String, Resolved)>,
    suffixes: Vec<(String, Resolved)>,
    /// Kunci yang diklaim lebih dari satu kategori, ditemukan saat membangun
    /// indeks. Ini pengganti langsung dari masalah "dua berkas tema kembar"
    /// yang harus dijaga sinkron secara manual.
    pub overlaps: Vec<String>,
}

impl Rules {
    /// Muat aturan dari `icons.toml` yang tertanam, lalu bangun indeksnya.
    /// Mengembalikan pesan galat yang menunjuk pelakunya kalau data ditolak.
    pub fn load() -> Result<Self, String> {
        Self::load_from(ICONS_TOML)
    }

    /// Memuat dari teks mana pun. `load()` memakai `ICONS_TOML`; tes memakai
    /// potongan buatan untuk membuktikan tiap penolakan benar-benar bekerja.
    fn load_from(teks: &str) -> Result<Self, String> {
        let file: IconsFile = toml::from_str(teks).map_err(|error| {
            // Pesan parse bisa menyertakan cuplikan baris — jangan sampai
            // membawa karakter kontrol mentah ke terminal.
            format!(
                "icons.toml tidak bisa dibaca: {}",
                render::sanitize(&error.to_string())
            )
        })?;

        if file.palette.is_empty() {
            return Err("palet di icons.toml kosong".to_string());
        }
        if file.families.is_empty() {
            return Err("tabel [families] di icons.toml kosong".to_string());
        }

        // Karakter kontrol/bidi di string mana pun = jebakan terminal bagi
        // siapa pun yang menjalankan `--list`/`--gallery`. Ditolak di sini,
        // sebelum pesan galat lain sempat menuliskan stringnya mentah — satu
        // gerbang ini menutup `load()`, `cargo test`, dan `--audit` sekaligus.
        let kontrol = karakter_terlarang(&file);
        if !kontrol.is_empty() {
            return Err(format!(
                "icons.toml ditolak, memuat karakter terlarang:\n  - {}",
                kontrol.join("\n  - ")
            ));
        }

        // Kode palet masuk ke `\x1b[{kode}m` apa adanya, jadi harus angka;
        // nama kuncinya sudah aman di langkah sebelumnya.
        for (nama, kode) in &file.palette {
            if kode.parse::<u16>().is_err() {
                return Err(format!(
                    "kode palet '{nama}' = '{}' bukan angka SGR yang sah",
                    render::sanitize(kode)
                ));
            }
        }

        for (family, color) in &file.families {
            if !file.palette.contains_key(color) {
                return Err(format!(
                    "keluarga '{family}' memakai warna '{color}', yang tidak ada di [palette]"
                ));
            }
        }
        for (name, category) in &file.categories {
            if !file.families.contains_key(&category.family) {
                return Err(format!(
                    "kategori '{name}' memakai keluarga '{}' yang tidak ada di [families]",
                    category.family
                ));
            }
        }
        for (name, rule) in &file.dirs {
            if !file.families.contains_key(&rule.family) {
                return Err(format!(
                    "folder '{name}' memakai keluarga '{}' yang tidak ada di [families]",
                    rule.family
                ));
            }
        }

        // `resolve_dir` selalu mencari dengan huruf kecil, jadi "Src" dan
        // "src" adalah kunci yang sama — `collect` akan menimpa salah satunya
        // tanpa suara. Tabrakan seperti itu ditolak di sini.
        let mut dirs = BTreeMap::new();
        for (name, rule) in file.dirs {
            let key = name.to_lowercase();
            if dirs.insert(key, rule).is_some() {
                return Err(format!(
                    "folder '{name}' di [dirs] bentrok dengan folder lain \
                     setelah huruf besar/kecil diseragamkan"
                ));
            }
        }

        let mut rules = Self {
            palette: file.palette,
            families: file.families,
            categories: file.categories,
            dirs,
            by_name: BTreeMap::new(),
            by_ext: BTreeMap::new(),
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            overlaps: Vec::new(),
        };

        for key in [DEFAULT_DIR_CATEGORY, DEFAULT_FILE_CATEGORY] {
            if !rules.categories.contains_key(key) {
                return Err(format!("kategori bawaan '{key}' tidak ada di icons.toml"));
            }
        }

        rules.build_index();
        Ok(rules)
    }

    fn build_index(&mut self) {
        let categories: Vec<(String, Category)> = self
            .categories
            .iter()
            .map(|(name, category)| (name.clone(), category.clone()))
            .collect();

        for (name, category) in &categories {
            let color = self.color_of(&category.family).to_string();

            for file_name in &category.names {
                let key = file_name.to_lowercase();
                if self.by_name.contains_key(&key) {
                    self.overlaps.push(format!(
                        "nama '{key}' diklaim dua kategori, salah satunya '{name}'"
                    ));
                    continue;
                }
                let glyph =
                    override_for(category, file_name).unwrap_or_else(|| category.glyph.clone());
                self.by_name
                    .insert(key, Resolved::new(glyph, color.clone()));
            }

            for ext in &category.ext {
                let key = ext.to_lowercase();
                if self.by_ext.contains_key(&key) {
                    self.overlaps.push(format!(
                        "ekstensi '{key}' diklaim dua kategori, salah satunya '{name}'"
                    ));
                    continue;
                }
                let glyph = override_for(category, ext).unwrap_or_else(|| category.glyph.clone());
                self.by_ext.insert(key, Resolved::new(glyph, color.clone()));
            }

            for prefix in &category.prefix {
                self.prefixes.push((
                    prefix.to_lowercase(),
                    Resolved::new(category.glyph.clone(), color.clone()),
                ));
            }

            for suffix in &category.suffix {
                self.suffixes.push((
                    suffix.to_lowercase(),
                    Resolved::new(category.glyph.clone(), color.clone()),
                ));
            }
        }

        sort_longest_first(&mut self.prefixes);
        sort_longest_first(&mut self.suffixes);
    }

    /// Ikon untuk nama folder, siap pakai. `matched_by` menjadi
    /// [`MatchedBy::WellKnownFolder`] kalau namanya dikenal, selain itu
    /// [`MatchedBy::Fallback`].
    pub fn resolve_dir(&self, name: &str) -> Resolved {
        let key = name.to_lowercase();
        match self.dirs.get(&key) {
            Some(rule) => Resolved {
                glyph: rule.glyph.clone(),
                color: self.color_of(&rule.family).to_string(),
                matched_by: MatchedBy::WellKnownFolder,
            },
            None => self.category(DEFAULT_DIR_CATEGORY),
        }
    }

    /// Satu-satunya tempat urutan prioritas ditetapkan, supaya `resolve_*` dan
    /// `explain_*` tidak bisa berbeda pendapat: nama persis → akhiran → awalan
    /// → ekstensi terpanjang. Kalau tidak ada yang cocok, hasilnya
    /// [`MatchedBy::Fallback`].
    fn cari(&self, key: &str) -> Option<(Resolved, MatchedBy, String)> {
        if let Some(resolved) = self.by_name.get(key) {
            return Some((resolved.clone(), MatchedBy::Name, key.to_string()));
        }
        if let Some((pattern, resolved)) =
            first_match(key, &self.suffixes, |name, p| name.ends_with(p))
        {
            return Some((resolved, MatchedBy::Suffix, pattern.to_string()));
        }
        if let Some((pattern, resolved)) =
            first_match(key, &self.prefixes, |name, p| name.starts_with(p))
        {
            return Some((resolved, MatchedBy::Prefix, pattern.to_string()));
        }
        if let Some((pattern, resolved)) = self.match_extension(key) {
            return Some((resolved, MatchedBy::Extension, pattern.to_string()));
        }
        None
    }

    /// Urutan pencarian, dari yang paling spesifik:
    /// nama persis → akhiran → awalan → ekstensi → cadangan.
    ///
    /// Akhiran didahulukan atas ekstensi supaya `foo.test.ts` terbaca sebagai
    /// berkas uji, bukan sekadar TypeScript.
    pub fn resolve_file(&self, name: &str) -> Resolved {
        let key = name.to_lowercase();
        match self.cari(&key) {
            Some((mut resolved, matched_by, _)) => {
                resolved.matched_by = matched_by;
                resolved
            }
            None => self.category(DEFAULT_FILE_CATEGORY),
        }
    }

    /// Bentuk siap pakai untuk TUI: karakter + nama warna + asal aturan.
    /// `ch` bernilai `None` hanya bila nama glyph-nya tidak ada di
    /// `assets/glyphs.toml`.
    pub fn icon_for(&self, glyphs: &Glyphs, name: &str) -> Icon {
        self.jadi_icon(glyphs, self.resolve_file(name))
    }

    /// Seperti [`Rules::icon_for`], untuk nama folder.
    pub fn icon_for_dir(&self, glyphs: &Glyphs, name: &str) -> Icon {
        self.jadi_icon(glyphs, self.resolve_dir(name))
    }

    fn jadi_icon(&self, glyphs: &Glyphs, resolved: Resolved) -> Icon {
        Icon {
            ch: glyphs.get(&resolved.glyph),
            color: resolved.color,
            matched_by: resolved.matched_by,
        }
    }

    /// Semua aturan yang cocok untuk nama berkas, berurutan prioritas.
    /// Kosong berarti tidak ada aturan yang cocok dan kategori bawaan yang
    /// dipakai. Inilah juga yang membuat `ikon --explain` bisa menunjukkan
    /// alasan, bukan cuma hasil.
    pub fn explain_file(&self, name: &str) -> Explanation {
        let key = name.to_lowercase();
        let mut kandidat = Vec::new();

        if let Some(resolved) = self.by_name.get(&key) {
            kandidat.push(Kandidat {
                matched_by: MatchedBy::Name,
                pattern: key.clone(),
                glyph: resolved.glyph.clone(),
                color: resolved.color.clone(),
            });
        }
        for (pola, resolved) in &self.suffixes {
            if key.ends_with(pola.as_str()) {
                kandidat.push(Kandidat {
                    matched_by: MatchedBy::Suffix,
                    pattern: pola.clone(),
                    glyph: resolved.glyph.clone(),
                    color: resolved.color.clone(),
                });
            }
        }
        for (pola, resolved) in &self.prefixes {
            if key.starts_with(pola.as_str()) {
                kandidat.push(Kandidat {
                    matched_by: MatchedBy::Prefix,
                    pattern: pola.clone(),
                    glyph: resolved.glyph.clone(),
                    color: resolved.color.clone(),
                });
            }
        }
        for (position, _) in key.match_indices('.') {
            if let Some(resolved) = self.by_ext.get(&key[position..]) {
                kandidat.push(Kandidat {
                    matched_by: MatchedBy::Extension,
                    pattern: key[position..].to_string(),
                    glyph: resolved.glyph.clone(),
                    color: resolved.color.clone(),
                });
            }
        }
        Explanation { kandidat }
    }

    /// Semua aturan yang cocok untuk nama folder, berurutan prioritas.
    pub fn explain_dir(&self, name: &str) -> Explanation {
        let key = name.to_lowercase();
        let mut kandidat = Vec::new();
        if let Some(rule) = self.dirs.get(&key) {
            kandidat.push(Kandidat {
                matched_by: MatchedBy::WellKnownFolder,
                pattern: key.clone(),
                glyph: rule.glyph.clone(),
                color: self.color_of(&rule.family).to_string(),
            });
        }
        Explanation { kandidat }
    }

    /// Ekstensi terpanjang yang menang, supaya "arsip.tar.gz" tidak jatuh ke
    /// ".gz" biasa dan "tipe.d.ts" menang atas ".ts". Potongan yang dikembalikan
    /// dipinjam dari `key`.
    fn match_extension<'a>(&self, key: &'a str) -> Option<(&'a str, Resolved)> {
        for (position, _) in key.match_indices('.') {
            if let Some(resolved) = self.by_ext.get(&key[position..]) {
                return Some((&key[position..], resolved.clone()));
            }
        }
        None
    }

    /// Aturan satu kategori untuk semua nama, dipakai untuk kategori bawaan.
    /// Kategori yang tidak ada menghasilkan glyph kosong dan warna `dim`.
    pub fn category(&self, name: &str) -> Resolved {
        self.categories
            .get(name)
            .map(|category| {
                Resolved::new(
                    category.glyph.clone(),
                    self.color_of(&category.family).to_string(),
                )
            })
            .unwrap_or_else(|| Resolved::new("", "dim"))
    }

    /// Kode SGR ANSI untuk satu nama warna, atau `None` kalau nama itu tidak
    /// ada di `[palette]`.
    pub fn ansi(&self, color: &str) -> Option<&str> {
        self.palette.get(color).map(String::as_str)
    }

    /// Warna sebuah keluarga. `load()` sudah menjamin semua keluarga yang
    /// disebut aturan ada di tabel, jadi fallback "dim" di sini hanya jaring
    /// pengaman — bukan keputusan warna.
    pub fn color_of(&self, family: &str) -> &str {
        self.families
            .get(family)
            .map(String::as_str)
            .unwrap_or("dim")
    }

    /// Jumlah aturan (kategori + folder) yang memakai warna `dim`. Satu
    /// sumber angka ini dipakai audit, tes, dan angka yang dicetak
    /// `ikon --audit` — supaya yang dicek dan yang dilaporkan tak bisa
    /// berbeda karena salinan yang lupa diperbarui.
    pub fn dim_rules(&self) -> usize {
        self.categories
            .values()
            .filter(|category| self.color_of(&category.family) == "dim")
            .count()
            + self
                .dirs
                .values()
                .filter(|rule| self.color_of(&rule.family) == "dim")
                .count()
    }

    /// Pemeriksaan konsistensi. Dipakai oleh `cargo test` DAN oleh
    /// `ikon --audit`, jadi masalah pemetaan ketahuan sebelum dirilis, bukan
    /// setelah ada yang sadar ikonnya kosong.
    pub fn audit(&self, glyphs: &Glyphs) -> Vec<String> {
        let mut findings = self.overlaps.clone();

        let check = |label: &str, glyph: &str, color: &str| -> Vec<String> {
            let mut problems = Vec::new();
            if !ALLOWED_FAMILIES
                .iter()
                .any(|family| glyph.starts_with(family))
            {
                problems.push(format!(
                    "{label}: glyph '{glyph}' bukan dari keluarga {}",
                    ALLOWED_FAMILIES.join(" atau ")
                ));
            }
            if glyphs.get(glyph).is_none() {
                problems.push(format!("{label}: glyph '{glyph}' tidak ada di tabel glyph"));
            }
            if !self.palette.contains_key(color) {
                problems.push(format!(
                    "{label}: warna '{color}' tidak ada di palet ({})",
                    self.palette.keys().cloned().collect::<Vec<_>>().join(", ")
                ));
            }
            problems
        };

        let mut used: BTreeSet<&str> = BTreeSet::new();

        for (name, category) in &self.categories {
            let label = format!("kategori '{name}'");
            findings.extend(check(
                &label,
                &category.glyph,
                self.color_of(&category.family),
            ));
            used.insert(category.glyph.as_str());

            for ext in &category.ext {
                if !ext.starts_with('.') {
                    findings.push(format!("{label}: ekstensi '{ext}' tidak diawali titik"));
                }
                if ext != &ext.to_lowercase() {
                    findings.push(format!("{label}: ekstensi '{ext}' tidak huruf kecil"));
                }
            }

            for (ext, glyph) in &category.by_ext {
                if !category.ext.contains(ext) {
                    findings.push(format!(
                        "{label}: bentuk khusus '{ext}' tidak ada di daftar `ext`"
                    ));
                }
                findings.extend(check(
                    &format!("{label} → {ext}"),
                    glyph,
                    self.color_of(&category.family),
                ));
                used.insert(glyph.as_str());
            }
        }

        for (name, rule) in &self.dirs {
            findings.extend(check(
                &format!("folder '{name}'"),
                &rule.glyph,
                self.color_of(&rule.family),
            ));
            used.insert(rule.glyph.as_str());
        }

        // --- Awalan/akhiran yang diklaim dua kategori --------------------------
        // Pemenangnya dipilih `build_index` diam-diam (urut alfabet kategori),
        // jadi audit yang harus menagih: dua kategori berebut satu pola berarti
        // keputusannya kebetulan, bukan sadar.
        let mut awalan: BTreeMap<String, String> = BTreeMap::new();
        for (name, category) in &self.categories {
            for pattern in &category.prefix {
                if let Some(other) = awalan.insert(pattern.to_lowercase(), name.clone()) {
                    findings.push(format!(
                        "awalan '{pattern}' diklaim kategori '{other}' dan '{name}'"
                    ));
                }
            }
        }
        let mut akhiran: BTreeMap<String, String> = BTreeMap::new();
        for (name, category) in &self.categories {
            for pattern in &category.suffix {
                if let Some(other) = akhiran.insert(pattern.to_lowercase(), name.clone()) {
                    findings.push(format!(
                        "akhiran '{pattern}' diklaim kategori '{other}' dan '{name}'"
                    ));
                }
            }
        }

        // --- Keluarga warna ---------------------------------------------------
        // Satu keluarga satu warna: kalau dua keluarga berebut satu warna,
        // "siapa yang berbagi warna" tidak lagi bisa dibaca dari nama keluarga.
        let mut per_warna: BTreeMap<&str, &str> = BTreeMap::new();
        for (family, color) in &self.families {
            if let Some(other) = per_warna.insert(color.as_str(), family.as_str()) {
                findings.push(format!(
                    "keluarga '{other}' dan '{family}' berbagi warna '{color}' — satu warna harus satu keluarga"
                ));
            }
        }
        for color in self.palette.keys() {
            if !self.families.values().any(|value| value == color) {
                findings.push(format!(
                    "warna '{color}' ada di [palette] tapi tidak dipakai keluarga mana pun"
                ));
            }
        }
        for family in self.families.keys() {
            let dipakai = self.categories.values().any(|c| &c.family == family)
                || self.dirs.values().any(|r| &r.family == family);
            if !dipakai {
                findings.push(format!("keluarga '{family}' tidak punya anggota satu pun"));
            }
        }

        // --- Ketetanggaan -----------------------------------------------------
        // Aturan yang bersebelahan di icons.toml hanya boleh sewarna kalau
        // satu keluarga. Lintas keluarga yang sewarna = pembagian warna kabur.
        for pasangan in urutan_dari_teks(ICONS_TOML).windows(2) {
            let (nama_a, keluarga_a) = &pasangan[0];
            let (nama_b, keluarga_b) = &pasangan[1];
            let (warna_a, warna_b) = (self.color_of(keluarga_a), self.color_of(keluarga_b));
            if warna_a == warna_b && keluarga_a != keluarga_b {
                findings.push(format!(
                    "'{nama_a}' dan '{nama_b}' bersebelahan di icons.toml tapi beda keluarga \
                     ('{keluarga_a}' vs '{keluarga_b}') dengan warna sama '{warna_a}'"
                ));
            }
        }

        // --- Batas dim ----------------------------------------------------------
        let dim = self.dim_rules();
        if dim > MAX_DIM_RULES {
            findings.push(format!(
                "pemakaian 'dim' mencapai {dim} aturan, batasnya {MAX_DIM_RULES} — \
                 'dim' hanya untuk berkas tak dikenal dan artefak hasil generate"
            ));
        }

        // Glyph yang tersedia di tabel tapi tidak dipakai = bobot mati.
        for name in glyphs.names() {
            if !used.contains(name) {
                findings.push(format!("glyph '{name}' ada di tabel tapi tidak dipakai"));
            }
        }

        findings
    }
}

fn override_for(category: &Category, key: &str) -> Option<String> {
    category.by_ext.get(key).cloned()
}

/// Semua string `icons.toml` yang bisa tercetak ke terminal diperiksa satu per
/// satu; hasilnya daftar temuan, pemanggil yang menyusun pesan. Nama bagian
/// ikut disanitasi supaya pesan galat sendiri tidak jadi serangan.
fn karakter_terlarang(file: &IconsFile) -> Vec<String> {
    let mut temuan: Vec<String> = Vec::new();
    let mut periksa = |apa: &str, nilai: &str| {
        if !render::is_terminal_safe(nilai) {
            temuan.push(format!(
                "{} memuat karakter kontrol/bidi: {}",
                render::sanitize(apa),
                render::sanitize(nilai)
            ));
        }
    };
    for (nama, kode) in &file.palette {
        periksa("nama palet", nama);
        periksa("kode palet", kode);
    }
    for (nama, warna) in &file.families {
        periksa("nama keluarga", nama);
        periksa("warna keluarga", warna);
    }
    for (nama, kategori) in &file.categories {
        periksa("nama kategori", nama);
        periksa(&format!("glyph kategori '{nama}'"), &kategori.glyph);
        for nilai in kategori
            .ext
            .iter()
            .chain(&kategori.names)
            .chain(&kategori.prefix)
            .chain(&kategori.suffix)
        {
            periksa(&format!("aturan kategori '{nama}'"), nilai);
        }
        for (ekstensi, glyph) in &kategori.by_ext {
            periksa(&format!("kunci by_ext kategori '{nama}'"), ekstensi);
            periksa(&format!("glyph by_ext kategori '{nama}'"), glyph);
        }
    }
    for (nama, aturan) in &file.dirs {
        periksa("nama folder", nama);
        periksa(&format!("glyph folder '{nama}'"), &aturan.glyph);
    }
    temuan
}

fn sort_longest_first(rules: &mut [(String, Resolved)]) {
    rules.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
}

/// Aturan pertama yang cocok, beserta polanya. Mengembalikan `(pola, hasil)`
/// supaya `explain_*` bisa menampilkan pola yang benar-benar menang. Pola yang
/// dikembalikan dipinjam dari daftar aturan, bukan dari `key`.
fn first_match<'a>(
    key: &str,
    rules: &'a [(String, Resolved)],
    predicate: impl Fn(&str, &str) -> bool,
) -> Option<(&'a str, Resolved)> {
    rules
        .iter()
        .find(|(pattern, _)| predicate(key, pattern.as_str()))
        .map(|(pattern, resolved)| (pattern.as_str(), resolved.clone()))
}

/// Ambil nilai string dari baris bergaya `key = "value"`.
fn nilai(baris: &str, kunci: &str) -> Option<String> {
    let sisa = baris.strip_prefix(kunci)?.trim_start();
    let sisa = sisa.strip_prefix('=')?.trim_start();
    let sisa = sisa.strip_prefix('"')?;
    let ujung = sisa.find('"')?;
    Some(sisa[..ujung].to_string())
}

/// Keluarga pada sebuah baris: baik bergaya `family = "x"` (kategori)
/// maupun tertanam di tabel inline `... family = "x" }` (folder khusus).
fn family_di_baris(baris: &str) -> Option<String> {
    let pos = baris.find("family =")?;
    nilai(&baris[pos..], "family")
}

/// Urutan aturan persis seperti tertulis di icons.toml: (nama, keluarga).
/// Pemeriksaan ketetanggaan harus membaca teks aslinya, sebab urutan
/// editorial itu tidak selamat saat file di-parse menjadi BTreeMap.
fn urutan_dari_teks(teks: &str) -> Vec<(String, String)> {
    let mut hasil: Vec<(String, String)> = Vec::new();
    let mut seksi = "";

    for baris in teks.lines() {
        let baris = baris.trim();
        // Komentar tidak ikut diparse — termasuk `family =` yang tertulis di
        // dalamnya, supaya parser urutan tidak salah baca di masa depan.
        if baris.starts_with('#') {
            continue;
        }
        let baris = match baris.find('#') {
            Some(pos) => baris[..pos].trim_end(),
            None => baris,
        };
        if baris == "[dirs]" {
            seksi = "dirs";
            continue;
        }
        if let Some(rest) = baris.strip_prefix("[categories.") {
            if let Some(nama) = rest.strip_suffix(']') {
                if nama.ends_with(".by_ext") {
                    continue; // sub-blok bentuk khusus: tetap di kategori yang sama
                }
                seksi = "kategori";
                hasil.push((nama.to_string(), String::new()));
                continue;
            }
        }
        if baris.starts_with('[') {
            seksi = "";
            continue;
        }
        let Some(keluarga) = family_di_baris(baris) else {
            continue;
        };
        match seksi {
            "dirs" => {
                let nama = baris
                    .split('=')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_string();
                if !nama.is_empty() {
                    hasil.push((nama, keluarga));
                }
            }
            "kategori" => {
                if let Some(akhir) = hasil.last_mut() {
                    akhir.1 = keluarga;
                }
            }
            _ => {}
        }
    }
    hasil
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        Rules::load().expect("icons.toml harus bisa dibaca")
    }

    fn glyph(name: &str) -> String {
        rules().resolve_file(name).glyph
    }

    fn color(name: &str) -> String {
        rules().resolve_file(name).color
    }

    /// Batas "minimalis" yang ditegakkan mesin: delapan slot warna. Menambah
    /// warna kesembilan akan menggagalkan test ini, yang memaksa keputusan itu
    /// diambil sadar, bukan kebetulan.
    #[test]
    fn palet_tidak_lebih_dari_delapan_warna() {
        let rules = rules();
        assert!(
            rules.palette.len() <= 8,
            "palet punya {} warna: {}",
            rules.palette.len(),
            rules.palette.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    /// Semua aturan harus konsisten: glyph ada, keluarga benar, warna dikenal,
    /// tidak ada kunci yang diklaim dua kategori.
    #[test]
    fn pemetaan_konsisten() {
        let findings = rules().audit(&Glyphs::bundled());
        assert!(
            findings.is_empty(),
            "audit menemukan {} masalah:\n  - {}",
            findings.len(),
            findings.join("\n  - ")
        );
    }

    #[test]
    fn bawaan_folder_dan_berkas() {
        let rules = rules();
        assert_eq!(
            rules.resolve_dir("folder-apa-saja").glyph,
            "nf-md-folder_outline"
        );
        assert_eq!(rules.resolve_file("berkas.xyz").glyph, "nf-md-file_outline");
        assert_eq!(rules.resolve_file("berkas.xyz").color, "dim");
    }

    #[test]
    fn folder_well_known_menang_atas_bawaan() {
        let rules = rules();
        assert_eq!(rules.resolve_dir(".git").glyph, "nf-md-source_branch");
        assert_eq!(rules.resolve_dir("node_modules").color, "dim");
        assert_eq!(rules.resolve_dir("SRC").glyph, "nf-md-code_braces");
    }

    #[test]
    fn bentuk_berbeda_di_dalam_kategori_yang_sama() {
        // Bentuknya beda per bahasa, warnanya harus tetap satu.
        assert_eq!(glyph("main.go"), "nf-md-language_go");
        assert_eq!(glyph("lib.rs"), "nf-md-language_rust");
        assert_eq!(color("main.go"), color("lib.rs"));
        assert_eq!(color("main.go"), "cyan");
    }

    #[test]
    fn ekstensi_terpanjang_yang_menang() {
        assert_eq!(glyph("backup.tar.gz"), "nf-md-folder_zip_outline");
        assert_eq!(glyph("tipe.d.ts"), "nf-md-language_typescript");
        assert_eq!(glyph("arsip.zip"), "nf-md-folder_zip_outline");
    }

    #[test]
    fn akhiran_didahulukan_atas_ekstensi() {
        // foo.test.ts harus terbaca sebagai berkas uji, bukan sekadar TypeScript.
        assert_eq!(glyph("foo.test.ts"), "nf-md-test_tube");
        assert_eq!(glyph("main_test.go"), "nf-md-test_tube");
        assert_eq!(glyph("main.go"), "nf-md-language_go");
    }

    #[test]
    fn nama_persis_didahulukan() {
        assert_eq!(glyph("go.mod"), "nf-md-code_json");
        assert_eq!(glyph("Dockerfile"), "nf-md-docker");
        assert_eq!(glyph("dockerfile"), "nf-md-docker");
        assert_eq!(glyph("CMakeLists.txt"), "nf-md-code_braces");
    }

    #[test]
    fn awalan_menangkap_berkas_dokumentasi() {
        assert_eq!(glyph("README.md"), "nf-md-book_open_page_variant");
        assert_eq!(glyph("readme"), "nf-md-book_open_page_variant");
        assert_eq!(glyph("LICENSE"), "nf-md-license");
        assert_eq!(glyph("CHANGELOG.md"), "nf-md-checkbox_marked_outline");
    }

    #[test]
    fn berkas_tersembunyi_punya_aturan_sendiri() {
        assert_eq!(glyph(".gitignore"), "nf-md-source_branch");
        assert_eq!(glyph(".env"), "nf-md-shield_key_outline");
        assert_eq!(glyph(".env.production"), "nf-md-shield_key_outline");
    }

    #[test]
    fn pencocokan_tidak_peduli_huruf_besar_kecil() {
        assert_eq!(glyph("MAIN.GO"), glyph("main.go"));
        assert_eq!(glyph("DockerFile"), glyph("dockerfile"));
    }

    /// Tes yang diminta: kategori yang bersebelahan di icons.toml tidak boleh
    /// memakai warna sama kecuali memang satu keluarga. Dua keluarga berbeda
    /// yang sewarna membuat pembagian warna kabur — dan gagal di sini.
    #[test]
    fn kategori_berdekatan_tidak_silang_keluarga() {
        let rules = rules();
        let urutan = urutan_dari_teks(ICONS_TOML);
        assert!(
            urutan.len() > 10,
            "pemindaian urutan icons.toml hanya menghasilkan {} aturan",
            urutan.len()
        );

        let mut pelanggaran = Vec::new();
        for pasangan in urutan.windows(2) {
            let (a, fa) = &pasangan[0];
            let (b, fb) = &pasangan[1];
            let (wa, wb) = (rules.color_of(fa), rules.color_of(fb));
            if wa == wb && fa != fb {
                pelanggaran.push(format!("{a} | {b}: warna '{wa}' tapi '{fa}' vs '{fb}'"));
            }
        }
        assert!(
            pelanggaran.is_empty(),
            "{} pasangan tetangga silang keluarga:\n  - {}",
            pelanggaran.len(),
            pelanggaran.join("\n  - ")
        );
    }

    /// Invarian keluarga, ditegakkan di dua lapis: tes ini menjelaskan
    /// maksudnya secara eksplisit, audit memeriksa semuanya (termasuk lewat
    /// `ikon --audit` sebelum rilis).
    #[test]
    fn satu_keluarga_satu_warna() {
        let rules = rules();

        let mut warna: Vec<&str> = rules.families.values().map(String::as_str).collect();
        warna.sort_unstable();
        let jumlah = warna.len();
        warna.dedup();
        assert_eq!(warna.len(), jumlah, "dua keluarga berebut satu warna");

        for (nama, kategori) in &rules.categories {
            assert!(
                rules.families.contains_key(&kategori.family),
                "kategori '{nama}' memakai keluarga '{}' yang tak ada di [families]",
                kategori.family
            );
        }
        for (nama, aturan) in &rules.dirs {
            assert!(
                rules.families.contains_key(&aturan.family),
                "folder '{nama}' memakai keluarga '{}' yang tak ada di [families]",
                aturan.family
            );
        }
        for warna in rules.palette.keys() {
            assert!(
                rules.families.values().any(|w| w == warna),
                "warna '{warna}' ada di [palette] tapi tidak dipakai keluarga mana pun"
            );
        }
    }

    /// `dim` berarti "tidak ada yang menarik di sini". Kalau dipakai segala
    /// macam, daftar terasa mati. Batasnya mesin, bukan selera.
    /// Urutan resolusi adalah kontrak pustaka, bukan sekadar internal: aplikasi
    /// TUI mengandalkan `matched_by` untuk menjelaskan pilihannya. Tes ini
    /// mengunci isi `Icon` sampai ke karakter dan nama warnanya.
    #[test]
    fn icon_siap_pakai_untuk_tui() {
        let rules = rules();
        let glyphs = Glyphs::bundled();

        let ikon = rules.icon_for(&glyphs, "app.test.ts");
        assert_eq!(ikon.matched_by, MatchedBy::Suffix);
        assert_eq!(
            ikon.color, "green",
            "warna harus nama palet, bukan kode ANSI"
        );
        assert!(ikon.ch.is_some(), "glyph harus ditemukan di tabel");

        let folder = rules.icon_for_dir(&glyphs, "src");
        assert_eq!(folder.matched_by, MatchedBy::WellKnownFolder);

        let ekstensi = rules.icon_for(&glyphs, "main.go");
        assert_eq!(ekstensi.matched_by, MatchedBy::Extension);

        // Tanpa aturan sama sekali: jatuh ke kategori bawaan, dan glyph
        // bawaannya tetap ada di tabel.
        let tanpa_aturan = rules.icon_for(&glyphs, "zzz");
        assert_eq!(tanpa_aturan.matched_by, MatchedBy::Fallback);
        assert!(tanpa_aturan.ch.is_some());
    }

    /// `explain_*` harus bisa intrigued alasan, termasuk aturan yang kalah — itulah
    /// yang menjawab "kenapa `latest.py` bukan ikon test?".
    #[test]
    fn explain_menampilkan_pemenang_dan_yang_kalah() {
        let rules = rules();

        let app = rules.explain_file("app.test.ts");
        let winner = app.winner().expect("harus ada pemenang");
        assert_eq!(winner.matched_by, MatchedBy::Suffix);
        assert_eq!(winner.pattern, ".test.ts");
        // Ekstensi juga cocok, tapi kalah prioritas terhadap akhiran.
        assert!(
            app.kandidat
                .iter()
                .any(|k| k.matched_by == MatchedBy::Extension && k.pattern == ".ts"),
            "ekstensi yang kalah harus tetap terlihat: {:?}",
            app.kandidat
        );

        let biasa = rules.explain_file("main.go");
        assert!(biasa
            .kandidat
            .iter()
            .all(|k| k.matched_by == MatchedBy::Extension));

        let folder = rules.explain_dir("node_modules");
        assert_eq!(
            folder.winner().map(|k| k.matched_by),
            Some(MatchedBy::WellKnownFolder)
        );

        let tak_ada = rules.explain_file("zzz.qqq");
        assert!(
            tak_ada.winner().is_none(),
            "tanpa aturan: winner harus None"
        );
    }

    #[test]
    fn pemakaian_dim_terbatas() {
        let rules = rules();
        let dim = rules.dim_rules();
        assert!(
            dim <= MAX_DIM_RULES,
            "pemakaian 'dim' mencapai {dim} aturan, batas {MAX_DIM_RULES}"
        );
    }

    /// Test penjaganya, bukan datanya. `pemetaan_konsisten` hanya membuktikan
    /// isi icons.toml bersih SEKARANG; kalau pemeriksaan di `audit` mati,
    /// test itu tetap lulus. Dua tes di bawah sengaja melanggar aturan dan
    /// memastikan audit benar-benar berbicara.
    #[test]
    fn audit_mendeteksi_keluarga_berebut_dan_tetangga_silang() {
        let mut rules = rules();

        // Ubah `netral` menjadi biru — kini berbagi warna dengan `dokumen`,
        // dan beberapa tetangga beda keluarga jadi sewarna.
        rules
            .families
            .insert("netral".to_string(), "blue".to_string());
        let findings = rules.audit(&Glyphs::bundled());

        assert!(
            findings.iter().any(|f| f.contains("berbagi warna")),
            "dua keluarga berebut warna tidak terdeteksi: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.contains("bersebelahan") && f.contains("doc")),
            "tetangga beda keluarga yang sewarna tidak terdeteksi: {findings:?}"
        );
    }

    #[test]
    fn audit_mendeteksi_dim_berlebih() {
        let mut rules = rules();

        // Arahkan keluarga besar ke `dim`: redup (9) + netral (8) = 17 aturan,
        // jauh di atas batas 10.
        rules
            .families
            .insert("netral".to_string(), "dim".to_string());
        let findings = rules.audit(&Glyphs::bundled());

        assert!(
            findings.iter().any(|f| f.contains("pemakaian 'dim'")),
            "pemakaian dim berlebih tidak terdeteksi: {findings:?}"
        );
    }

    /// Pola yang diklaim dua kategori dipilih pemenangnya diam-diam oleh
    /// `build_index`; tes ini menaruh `readme` (milik kategori lain) dan
    /// `_test.go` (milik `test`) di dalam `license` dan menuntut audit bicara.
    #[test]
    fn audit_mendeteksi_awalan_dan_akhiran_berebut() {
        let mut rules = rules();
        let license = rules
            .categories
            .get_mut("license")
            .expect("kategori license harus ada");
        license.prefix.push("readme".to_string());
        license.suffix.push("_test.go".to_string());

        let findings = rules.audit(&Glyphs::bundled());

        assert!(
            findings.iter().any(|f| f.contains("awalan 'readme'")),
            "awalan yang diklaim dua kategori tidak terdeteksi: {findings:?}"
        );
        assert!(
            findings.iter().any(|f| f.contains("akhiran '_test.go'")),
            "akhiran yang diklaim dua kategori tidak terdeteksi: {findings:?}"
        );
    }

    /// Fixture minimal yang sah — cukup untuk lolos semua cek struktural
    /// `load()`, supaya tes di bawah ini menyasar satu penolakan tertentu.
    fn teks_minimal() -> String {
        [
            "[palette]",
            "white = \"37\"",
            "",
            "[families]",
            "netral = \"white\"",
            "",
            "[categories.folder]",
            "glyph = \"nf-md-folder_outline\"",
            "family = \"netral\"",
            "",
            "[categories.file]",
            "glyph = \"nf-md-file_outline\"",
            "family = \"netral\"",
        ]
        .join("\n")
    }

    /// Gerbang F-001: `\u001b` di teks TOML menjadi karakter ESC sesungguhnya
    /// setelah di-parse — persis yang dulu lolos sampai ke stdout `--list`.
    #[test]
    fn load_menolak_karakter_kontrol() {
        let mut teks = teks_minimal();
        teks.push_str("\n\n[categories.kotor]\n");
        teks.push_str("glyph = \"nf-md-cached\"\n");
        teks.push_str("family = \"netral\"\n");
        teks.push_str("names = [\"\\u001b[31mPWN\\u001b[0m\"]\n");

        let Err(pesan) = Rules::load_from(&teks) else {
            panic!("load() menerima karakter kontrol/bidi");
        };
        assert!(pesan.contains("karakter kontrol"), "{pesan}");
        assert!(
            pesan.contains("\\u{1b}"),
            "pesan harus memakai representasi aman, bukan ESC mentah: {pesan}"
        );
        assert!(
            !pesan.contains('\u{1b}'),
            "pesan galat sendiri tidak boleh memuat ESC mentah"
        );
    }

    /// `resolve_dir` mencari dengan huruf kecil, jadi "src" dan "SRC" adalah
    /// kunci yang sama — tanpa penolakan ini, satu aturan ditimpa tanpa suara.
    #[test]
    fn load_menolak_folder_bedakapitalisasi() {
        let mut teks = teks_minimal();
        teks.push_str("\n\n[dirs]\n");
        teks.push_str("src = { glyph = \"nf-md-cached\", family = \"netral\" }\n");
        teks.push_str("SRC = { glyph = \"nf-md-cached\", family = \"netral\" }\n");

        let Err(pesan) = Rules::load_from(&teks) else {
            panic!("load() menerima dua folder yang beda kapitalisasi");
        };
        assert!(pesan.contains("bentrok"), "{pesan}");
    }

    /// Kode palet masuk ke `\x1b[{kode}m` apa adanya, jadi bentuknya divalidasi
    /// di batas muat — angka SGR, bukan string bebas.
    #[test]
    fn load_menolak_kode_palet_bukan_angka() {
        let teks = teks_minimal().replace("white = \"37\"", "white = \"3x\"");
        let Err(pesan) = Rules::load_from(&teks) else {
            panic!("load() menerima kode palet yang bukan angka SGR");
        };
        assert!(pesan.contains("SGR"), "{pesan}");
    }

    /// Akhiran tanpa pembatas ("test.py") dulu ikut menangkap "latest.py" dan
    /// "protest.cpp". Tes ini mengunci keputusannya: akhiran hanya yang
    /// berpembatas, nama literal hidup di `names`, dan konvensi baku
    /// (`FooTests.cs`, `FooSpec.scala`) tetap tertangkap.
    #[test]
    fn akhiran_tanpa_pembatas_jangan_menangkap_berkas_biasa() {
        // False positive yang dulu terjadi:
        assert_eq!(glyph("latest.py"), "nf-md-language_python");
        assert_eq!(glyph("protest.cpp"), "nf-md-language_cpp");
        assert_eq!(glyph("attest.rs"), "nf-md-language_rust");
        // Yang tetap harus tertangkap:
        assert_eq!(glyph("test.py"), "nf-md-test_tube");
        assert_eq!(glyph("foo_test.c"), "nf-md-test_tube");
        assert_eq!(glyph("FooTests.cs"), "nf-md-test_tube");
        assert_eq!(glyph("FooSpec.scala"), "nf-md-test_tube");
    }

    /// "Urutan resolusi" di `icons.toml` itu kontrak, bukan prosa. Setiap
    /// langkah diuji dengan nama yang hanya bisa dijawab benar oleh urutan itu,
    /// supaya kodenya tidak bisa menyimpang dari dokumentasinya.
    #[test]
    fn urutan_resolusi_ikut_dokumentasi() {
        // nama persis > awalan ("dockerfile" juga awalan kategori container)
        assert_eq!(glyph("dockerfile"), "nf-md-docker");
        // akhiran > awalan ("install." dan ".test.ts" sama-sama cocok)
        assert_eq!(glyph("install.test.ts"), "nf-md-test_tube");
        // awalan > ekstensi ("README.md" juga berakhir .md)
        assert_eq!(glyph("README.md"), "nf-md-book_open_page_variant");
        // ekstensi > bawaan (bukan nama berkas yang tak dikenal)
        assert_eq!(glyph("berkas.rs"), "nf-md-language_rust");
    }
}
