//! `ticon` — pustaka pemetaan ikon, plus perintah `ticon` untuk terminal.
//!
//! Dua hal dalam satu paket:
//!
//! * **Pustaka** `ticon`: memuat aturan dari `icons.toml` lalu menjawab
//!   "nama ini dapat glyph apa, warna apa, dan kenapa". Hasilnya
//!   ([`mapping::Icon`]) berisi karakter dan **nama** warna — bukan string
//!   ANSI — supaya pemanggil (TUI, skrip, aplikasi Rust lain) yang memilih
//!   cara mewarnainya sendiri.
//! * **Perintah** `ticon`: menampilkan direktori dengan ikon, lewat
//!   `--list`, `--gallery`, `--audit`, `--explain`, dan `--export`.
//!
//! ```no_run
//! let rules = ticon::mapping::Rules::load()?;
//! let glyphs = ticon::glyph::Glyphs::bundled();
//! let icon = rules.icon_for(&glyphs, "main.rs");
//! println!("{:?} {} {:?}", icon.ch, icon.color, icon.matched_by);
//! # Ok::<(), String>(())
//! ```
//!
//! Alur kerjanya sengaja dibikin sempit:
//!
//! ```text
//!   nama berkas ──► Rules::resolve_*  ──► nama glyph + nama warna
//!                                          │
//!                          Glyphs ─────────┴──► karakter + kode ANSI ──► render
//! ```
//!
//! Yang penting dari bagan itu: resolver TIDAK PERNAH tahu karakter apa yang
//! dipakai, dan renderer TIDAK PERNAH tahu aturan pencocokan. Semua aturan
//! hidup di `icons.toml`, semua codepoint hidup di `assets/glyphs.toml`, dan
//! kode di sini cuma menyambungkan keduanya.

#![warn(missing_docs)]

pub mod cli;
pub mod glyph;
pub mod json;
pub mod mapping;
pub mod pack;
pub mod peta;
pub mod raster_data;
pub mod render;
pub mod terminal;

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

use cli::{Command, Format, Options, Sort};
use glyph::Glyphs;
use mapping::{Prioritas, Rules};

/// Lebar sel yang diasumsikan untuk satu glyph Nerd Font kalau data tidak
/// menyebut lain.
///
/// **Nilai ini belum diukur** — ia warisan dari perilaku renderer lama, bukan
/// hasil pengukuran. Glyph Nerd Font bersifat *ambiguous-width*: banyak
/// terminal modern merendernya satu sel, sebagian lagi dua. Karena itu angka
/// ini hanya titik awal, dan **`ticon` sendiri tidak memakainya** untuk
/// merender: aplikasi yang tahu lebar glyph di terminalnya wajib mengukur
/// sendiri. Atur lewat `width` per aturan kalau punya jawaban yang lebih baik.
pub const LEBAR_GLIF_BAWAAN: usize = 1;

/// Alamat JSON Schema untuk kontrak ini. Dipakai di berkas ekspor supaya
/// editor dan validator bisa menautkannya; pemuat di `peta.rs` mengabaikan
/// kunci `$schema` (alat lain boleh menambahkannya).
pub const SKEMA_URL: &str = "https://ticon.pages.dev/schema/ticon-map-2.json";
use render::Item;
use unicode_width::UnicodeWidthStr;

use crate::pack::{Bentuk, Meta};

/// Jalankan perintah: parse argumen dari [`std::env::args`], kerjakan, lalu
/// kembalikan exit code-nya. Titik masuk perintah `ticon`.
pub fn run() -> ExitCode {
    match execute() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("ticon: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Semua icon yang dipakai di satu tampilan: peta, bentuk cadangan, dan pack
/// yang dipilih.
///
/// Bentuk sengaja di sini, bukan di dalam [`Rules`], karena bentuk cuma perlu
/// kalau glyph Nerd Font tidak bisa digambar — dan pemanggil TUI yang punya
/// font sendiri tidak pernahFQ perlu settling vecinya.
pub struct Muat {
    /// Peta ikon yang dipakai (bawaan, atau peta netral + pack).
    pub rules: Rules,
    /// Bentuk cadangan dari pack, kalau ada.
    pub bentuk: Bentuk,
    /// Metadata pack, kalau ada pack.
    pub meta: Option<Meta>,
}

impl Muat {
    /// Peta bawaan `ticon` saja, tanpa pack.
    pub fn bawaan() -> Result<Self, String> {
        Ok(Self {
            rules: Rules::load()?,
            bentuk: Bentuk::kosong(),
            meta: None,
        })
    }

    /// Peta netral dari berkas (`ticon-map/2`), tanpa pack.
    pub fn dari_peta(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        Ok(Self {
            rules: peta::rules_dari_berkas(path.as_ref())?,
            bentuk: Bentuk::kosong(),
            meta: None,
        })
    }

    /// Terapkan pack di atas peta yang sudah dimuat. Pack terakhir menang, dan
    /// pack bisa menimpa pack sebelumnya lewat aturan yang sama.
    pub fn dengan_pack(self, nama: &str) -> Result<Self, String> {
        let pack = pack::muat_nama(nama, &self.rules)?;
        Ok(Self {
            rules: pack.rules,
            bentuk: pack.bentuk,
            meta: Some(pack.meta),
        })
    }

    /// Bentuk cadangan untuk sebuah nama, kalau pack aktif menyediakannya.
    ///
    /// Ini jalur **bebas Nerd Font** untuk TUI: hasilnya `None` berarti "tidak
    /// ada ikon" - bukan karakter sembarang, dan bukan juga glyph Nerd Font.
    /// Pemanggil yang memakai fungsi ini sedang tidak ingin menampilkan kotak,
    /// jadi ia wajib memperlakukan `None` sebagai ketiadaan ikon.
    ///
    /// Kalau pemanggil lebih suka satu fungsi yang selalu mengembalikan
    /// sesuatu, pakai [`crate::mapping::Rules::icon_for_dengan_shape`] dan lihat
    /// kedua field `Icon` (`ch` dan `shape`) secara terpisah.
    pub fn bentuk_untuk(&self, nama: &str, folder: bool) -> Option<char> {
        let resolved = if folder {
            self.rules.resolve_dir(nama)
        } else {
            self.rules.resolve_file(nama)
        };
        self.bentuk.untuk(&self.rules, &resolved.color, folder)
    }
}

/// Muat peta ikon: dari `icons.toml` bawaan, atau dari peta netral yang diminta
/// pengguna dengan `--icons-map`, lalu digabung dengan icon pack kalau ada.
/// Satu pintu masuk supaya semua perintah memakai sumber yang sama.
fn muat_semua(map: &Option<PathBuf>, icons_pack: &Option<String>) -> Result<Muat, String> {
    let mut muat = match map {
        Some(path) => Muat::dari_peta(path)?,
        None => Muat::bawaan()?,
    };
    if let Some(nama) = icons_pack {
        muat = muat.dengan_pack(nama)?;
    }
    Ok(muat)
}

fn execute() -> Result<ExitCode, String> {
    // Bantuan dan versi tidak perlu membaca apa pun, jadi pemetaan baru
    // dimuat saat memang dibutuhkan.
    let command = cli::parse(std::env::args().skip(1));

    match command {
        Command::Help => {
            print!("{}", cli::HELP);
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("ticon {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        Command::ListPacks => {
            print_list_packs();
            Ok(ExitCode::SUCCESS)
        }
        Command::Error(message) => Err(format!("{message}\n\nCoba `ticon --help`")),
        Command::Audit(icons_map) => {
            let rules = muat_semua(&icons_map, &None)?.rules;
            let glyphs = Glyphs::bundled();
            audit(&rules, &glyphs)
        }
        Command::Explain(name, icons_map) => {
            let rules = muat_semua(&icons_map, &None)?.rules;
            let glyphs = Glyphs::bundled();
            print_explain(&rules, &glyphs, &name);
            Ok(ExitCode::SUCCESS)
        }
        Command::Export(format, icons_map) => {
            let rules = muat_semua(&icons_map, &None)?.rules;
            let glyphs = Glyphs::bundled();
            print_export(&rules, &glyphs, format);
            Ok(ExitCode::SUCCESS)
        }
        Command::Mapping(options) => {
            cek_tanpa_path("--list", &options)?;
            let muat = muat_semua(&options.icons_map, &options.icons_pack)?;
            let glyphs = Glyphs::bundled();
            print_mapping(&muat.rules, &glyphs, &options);
            Ok(ExitCode::SUCCESS)
        }
        Command::Gallery(options) => {
            cek_tanpa_path("--gallery", &options)?;
            let muat = muat_semua(&options.icons_map, &options.icons_pack)?;
            let glyphs = Glyphs::bundled();
            print_gallery(&muat.rules, &glyphs, &options, &muat.bentuk);
            Ok(ExitCode::SUCCESS)
        }
        Command::Dir(options) => {
            let muat = muat_semua(&options.icons_map, &options.icons_pack)?;
            let glyphs = Glyphs::bundled();
            list(&muat.rules, &glyphs, &options, &muat.bentuk)
        }
    }
}

/// Cetak pack yang ditemukan: nama, versi, asal, dan keterangan.
///
/// Kolomnya tab-separated, seperti `--export` TSV, supaya `ticon
/// --icons-list-packs | cut -f1` berguna untuk skrip.
fn print_list_packs() {
    let packs = pack::daftar();
    let lebar = packs
        .iter()
        .map(|info| UnicodeWidthStr::width(info.nama.as_str()))
        .chain(std::iter::once(UnicodeWidthStr::width("PACK")))
        .max()
        .unwrap_or(4);
    println!("{:<lebar$}\tASAL\tKETERANGAN", "PACK");

    for info in packs {
        // Sanitasi dilakukan per bagian, BUKAN pada baris yang sudah dirangkai.
        // `sanitize()` mengganti karakter kontrol dengan teks `\u{...}` - kalau
        // dipakai setelah tab disisipkan, pemisah kolom ikut berubah jadi
        // `\u{9}` dan kolomnya tidak bisa di-cut. (Ini memang terjadi di 0.4.3.)
        let meta = match &info.meta {
            Ok(meta) => {
                // `sanitize` mengembalikan `Cow`, jadi kumpulkan lewat
                // `to_mut()` - bukan `push`, yang tidak ada di `Cow`.
                let mut kolom = render::sanitize(&meta.versi).into_owned();
                if let Some(deskripsi) = &meta.deskripsi {
                    kolom.push('\t');
                    kolom.push_str(&render::sanitize(deskripsi));
                }
                if let Some(penulis) = &meta.penulis {
                    kolom.push('\t');
                    kolom.push('(');
                    kolom.push_str(&render::sanitize(penulis));
                    kolom.push(')');
                }
                kolom
            }
            // Pack yang ada tapi gagal dimuat tetap ditampilkan, lengkap dengan
            // alasannya: pack yang diam-diam hilang dari daftar lebih buruk
            // daripada pack yang terlihat rusak.
            Err(pesan) => format!("GAGAL DIMUAT\t{}", render::sanitize(pesan)),
        };
        println!("{:<lebar$}\t{}\t{}", info.nama, info.asal.label(), meta);
    }
}

// ---------------------------------------------------------------------------
//  Daftar berkas
// ---------------------------------------------------------------------------

struct Entry {
    name: String,
    is_dir: bool,
    link_target: Option<String>,
    size: u64,
    modified: Option<SystemTime>,
}

impl Entry {
    fn from_dir_entry(entry: &fs::DirEntry) -> Self {
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata().ok();
        let link_target = read_link_target(&entry.path());

        Self {
            name,
            is_dir: metadata.as_ref().is_some_and(|data| data.is_dir()),
            link_target,
            size: metadata.as_ref().map_or(0, |data| data.len()),
            modified: metadata.as_ref().and_then(|data| data.modified().ok()),
        }
    }

    fn from_path(path: &Path, metadata: &fs::Metadata) -> Self {
        Self {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            is_dir: metadata.is_dir(),
            link_target: read_link_target(path),
            size: metadata.len(),
            modified: metadata.modified().ok(),
        }
    }
}

fn read_link_target(path: &Path) -> Option<String> {
    fs::read_link(path)
        .ok()
        .map(|target| target.to_string_lossy().into_owned())
}

fn list(
    rules: &Rules,
    glyphs: &Glyphs,
    options: &Options,
    bentuk: &Bentuk,
) -> Result<ExitCode, String> {
    let paths = if options.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        options.paths.clone()
    };

    let show_headers = paths.len() > 1;
    let term_width = options.width.or_else(terminal::width).unwrap_or(80);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut failures = 0usize;

    for (index, path) in paths.iter().enumerate() {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) => {
                eprintln!("ticon: {}: {error}", path.display());
                failures += 1;
                continue;
            }
        };

        if show_headers {
            if index > 0 {
                let _ = writeln!(out);
            }
            let header = format!("{}:", path.display());
            let header = render::sanitize(&header);
            let _ = writeln!(
                out,
                "{}",
                render::paint(rules, options.color, "dim", &header)
            );
        }

        let items = if metadata.is_dir() {
            let mut entries = read_entries(path, options)?;
            sort_entries(&mut entries, options.sort);
            entries
                .iter()
                .map(|entry| render_entry(rules, glyphs, options, entry, bentuk))
                .collect()
        } else {
            vec![render_entry(
                rules,
                glyphs,
                options,
                &Entry::from_path(path, &metadata),
                bentuk,
            )]
        };

        let block = if options.one_per_line {
            items
                .iter()
                .map(|item| format!("{}\n", item.painted))
                .collect::<String>()
        } else {
            render::grid(&items, term_width)
        };
        let _ = out.write_all(block.as_bytes());
    }

    let _ = out.flush();
    Ok(if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn read_entries(path: &Path, options: &Options) -> Result<Vec<Entry>, String> {
    let directory = fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;

    let mut entries = Vec::new();
    for entry in directory {
        let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !options.all && name.starts_with('.') {
            continue;
        }
        entries.push(Entry::from_dir_entry(&entry));
    }
    Ok(entries)
}

/// Folder selalu di atas, apa pun kunci urutnya. Sisanya mengikuti `sort`.
fn sort_entries(entries: &mut [Entry], sort: Sort) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| match sort {
                Sort::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                Sort::Ext => extension_of(&a.name)
                    .cmp(&extension_of(&b.name))
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                Sort::Size => b.size.cmp(&a.size),
                Sort::Time => b.modified.cmp(&a.modified),
            })
            .then_with(|| a.name.cmp(&b.name))
    });
}

fn extension_of(name: &str) -> String {
    match name.rfind('.') {
        Some(position) if position > 0 => name[position + 1..].to_lowercase(),
        _ => String::new(),
    }
}

fn render_entry(
    rules: &Rules,
    glyphs: &Glyphs,
    options: &Options,
    entry: &Entry,
    bentuk: &Bentuk,
) -> Item {
    let resolved = if entry.is_dir {
        rules.resolve_dir(&entry.name)
    } else {
        rules.resolve_file(&entry.name)
    };

    let mut painted = String::new();
    let mut plain = String::new();

    if options.icons {
        // Bentuk pack menang kalau ada: kalau pengguna memilih pack `shape` atau
        // `sempit`, mereka memang meminta char itu — bukan glyph Nerd Font yang
        // tidak bisa digambar di terminal mereka.
        let glyph = bentuk
            .untuk(rules, &resolved.color, entry.is_dir)
            .or_else(|| glyphs.get(&resolved.glyph));
        if let Some(glyph) = glyph {
            let cell = format!("{glyph} ");
            plain.push_str(&cell);
            painted.push_str(&render::paint(rules, options.color, &resolved.color, &cell));
        }
    }

    // Hanya ikon yang diberi warna. Nama dibiarkan memakai warna teks bawaan
    // terminal supaya daftarnya tetap tenang — penanda jenis berkas sudah
    // dibawa oleh warna ikon. Nama berkas dari luar disanitasi dulu: satu
    // karakter ESC sudah cukup untuk menyuntikkan sekuens ke terminal.
    let name = render::sanitize(&entry.name);
    plain.push_str(&name);
    match entry.link_target {
        Some(_) => painted.push_str(&render::paint(rules, options.color, "cyan", &name)),
        None => painted.push_str(&name),
    }

    if let Some(target) = &entry.link_target {
        let suffix = format!(" → {}", render::sanitize(target));
        plain.push_str(&suffix);
        painted.push_str(&render::paint(rules, options.color, "dim", &suffix));
    }

    Item::styled(painted, &plain)
}

// ---------------------------------------------------------------------------
//  Tabel pemetaan dan audit
// ---------------------------------------------------------------------------

/// Galeri contoh: satu baris per aturan, dan contoh namanya benar-benar
/// dilewatkan ke resolver yang sama dengan yang dipakai `ticon` sehari-hari.
/// Jadi yang kamu lihat di sini bukan gambar atas nama desain — itu memang
/// hasil yang akan keluar.
fn print_gallery(rules: &Rules, glyphs: &Glyphs, options: &Options, bentuk: &Bentuk) {
    let mut examples: Vec<(String, String, bool)> = Vec::new();

    for name in rules.dirs.keys() {
        examples.push((name.clone(), format!("folder:{name}"), true));
    }

    for (name, category) in &rules.categories {
        if let Some(value) = category.names.first().or_else(|| category.prefix.first()) {
            examples.push((value.clone(), name.clone(), false));
        } else if let Some(value) = category.suffix.first() {
            examples.push((format!("berkas{value}"), name.clone(), false));
        }

        if let Some(extension) = category.ext.first() {
            examples.push((format!("berkas{extension}"), name.clone(), false));
        }

        for extension in category.by_ext.keys() {
            examples.push((format!("berkas{extension}"), name.clone(), false));
        }
    }

    let mut rows: Vec<(String, String, String, String, bool)> = Vec::new();
    for (example, label, is_dir) in examples {
        let resolved = if is_dir {
            rules.resolve_dir(&example)
        } else {
            rules.resolve_file(&example)
        };
        if resolved.glyph.is_empty() {
            continue;
        }
        rows.push((example, label, resolved.glyph, resolved.color, is_dir));
    }

    let example_width = rows
        .iter()
        .map(|row| UnicodeWidthStr::width(row.0.as_str()))
        .max()
        .unwrap_or(0);
    let label_width = rows
        .iter()
        .map(|row| UnicodeWidthStr::width(row.1.as_str()))
        .max()
        .unwrap_or(0);

    println!(
        "galeri ikon ({} aturan, {} glyph tertanam):",
        rows.len(),
        glyphs.len()
    );
    if !bentuk.is_kosong() {
        // Marketplace yang jujur: kalau pack yang dipakai hanya menimpa sebagian
        // keluarga, sebutkan itu. diam-diam menampilkan bentuk untuk semua
        // ikon akan membuat pengguna mengira pack-nya bekerja lebih luas dari
        // yang sebenarnya.
        println!(
            "bentuk: {} (dari pack; {} karakter)",
            bentuk.semua_karakter().len(),
            bentuk
                .semua_karakter()
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!();
    for (example, label, glyph_name, color, is_dir) in &rows {
        let character = bentuk
            .untuk(rules, color, *is_dir)
            .map(|value| value.to_string())
            .or_else(|| glyphs.get(glyph_name).map(|value| value.to_string()))
            .unwrap_or_else(|| "?".to_string());
        let icon = render::paint(rules, options.color, color, &character);
        // Pad dulu, baru warnai — kalau escape ANSI yang ikut dihitung saat
        // padding, kolom label lari 9 karakter saat warna aktif.
        let name = render::paint(rules, options.color, color, &pad(example, example_width));
        let swatch = render::paint(rules, options.color, color, color);
        println!("  {icon}  {name}  {label:<label_width$}  {swatch}");
    }
}

/// Tampilan untuk meninjau desain: satu baris per kategori, lalu rincian
/// aturannya menjorok di bawahnya. Sengaja dijaga agar tetap bisa di-grep —
/// `ticon --list | grep language_go` harus tetap berguna.
fn print_mapping(rules: &Rules, glyphs: &Glyphs, options: &Options) {
    // Lebar kolom nama ikut menghitung folder khusus dengan awalan "/";
    // tanpa ini baris seperti `/node_modules` bergeser melewati kolom glyph.
    let name_width = rules
        .categories
        .keys()
        .map(|name| UnicodeWidthStr::width(name.as_str()))
        .chain(
            rules
                .dirs
                .keys()
                .map(|name| 1 + UnicodeWidthStr::width(name.as_str())),
        )
        .max()
        .unwrap_or(0);
    let glyph_width = rules
        .categories
        .values()
        .flat_map(|category| {
            std::iter::once(category.glyph.clone()).chain(category.by_ext.values().cloned())
        })
        .chain(rules.dirs.values().map(|rule| rule.glyph.clone()))
        .map(|glyph| glyph.len())
        .max()
        .unwrap_or(0);
    let width = options.width.or_else(terminal::width).unwrap_or(100);

    println!("kategori ({}):", rules.categories.len());
    for (name, category) in &rules.categories {
        print_category_row(
            rules,
            glyphs,
            options,
            name,
            &category.glyph,
            rules.color_of(&category.family),
            name_width,
            glyph_width,
        );

        wrapped("nama", &sorted(&category.names), width);
        wrapped("awalan", &sorted(&category.prefix), width);
        wrapped("akhiran", &sorted(&category.suffix), width);
        wrapped("ekstensi", &sorted(&category.ext), width);

        let shapes: Vec<String> = category
            .by_ext
            .iter()
            .map(|(ext, glyph)| format!("{ext} → {glyph}"))
            .collect();
        wrapped("bentuk", &shapes, width);
    }

    println!();
    println!("folder khusus ({}):", rules.dirs.len());
    for (name, rule) in &rules.dirs {
        print_category_row(
            rules,
            glyphs,
            options,
            &format!("/{name}"),
            &rule.glyph,
            rules.color_of(&rule.family),
            name_width,
            glyph_width,
        );
    }

    println!();
    println!("keluarga warna ({}):", rules.families.len());
    let family_width = rules.families.keys().map(String::len).max().unwrap_or(0);
    for (family, color) in &rules.families {
        let jumlah = rules
            .categories
            .values()
            .filter(|category| &category.family == family)
            .count()
            + rules
                .dirs
                .values()
                .filter(|rule| &rule.family == family)
                .count();
        let swatch = render::paint(rules, options.color, color, "████");
        println!("  {family:<family_width$} = {color:<8} {swatch}  {jumlah} aturan");
    }

    println!();
    println!("palet ({} warna ANSI):", rules.palette.len());
    for (name, code) in &rules.palette {
        let sample = render::paint(rules, options.color, name, "████");
        println!("  {name:<8} \\x1b[{code}m  {sample}");
    }

    println!();
    println!("{} glyph tertanam, dari Nerd Fonts v3.4.0", glyphs.len());
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values
}

/// `--explain`: kenapa sebuah nama mendapat ikon itu.
///
/// Kalau nama itu benar-benar ada di disk, jenisnya diambil dari sana; kalau
/// tidak, diasumsikan berkas — dan asumsi itu ditulis di keluaran supaya tidak
/// menyesatkan.
fn print_explain(rules: &Rules, glyphs: &Glyphs, name: &str) {
    let is_dir = fs::metadata(name).is_ok_and(|meta| meta.is_dir());
    let (icon, penjelasan) = if is_dir {
        (rules.icon_for_dir(glyphs, name), rules.explain_dir(name))
    } else {
        (rules.icon_for(glyphs, name), rules.explain_file(name))
    };

    println!("{name}   ({})", if is_dir { "folder" } else { "berkas" });
    // Tanpa perataan: glyph Nerd Font occupying dua sel meski `unicode-width`
    // menghitungnya satu, jadi kolom yang "lurus" di sini justru menipu.
    println!(
        "  glyph  {}   warna {}",
        icon.ch
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(tidak ada di tabel)".to_string()),
        icon.color
    );

    match penjelasan.winner() {
        Some(winner) => {
            println!(
                "  menang  {:<16} {:?}",
                winner.matched_by.label(),
                winner.pattern
            );
            for kalah in penjelasan.kandidat.iter().skip(1) {
                println!(
                    "  kalah   {:<16} {:?}",
                    kalah.matched_by.label(),
                    kalah.pattern
                );
            }
            if penjelasan.kandidat.len() == 1 {
                println!("  (tidak ada aturan lain yang cocok)");
            }
        }
        None => {
            let bawaan = if is_dir {
                mapping::DEFAULT_DIR_CATEGORY
            } else {
                mapping::DEFAULT_FILE_CATEGORY
            };
            println!("  (tidak ada aturan yang cocok; jatuh ke kategori bawaan `{bawaan}`)");
        }
    }
}

/// Satu aturan yang siap diekspor, netral terhadap bentuk keluaran: TSV dan
/// JSON membaca daftar yang sama, jadi keduanya tidak mungkin berbeda isi.
struct AturanEkspor {
    /// `None` untuk folder well-known: itu bukan tahap resolusi berkas.
    prioritas: Option<Prioritas>,
    /// Nilai `kind` di kontrak: `name`, `suffix`, `prefix`, `ext`, atau `dir`.
    kind: &'static str,
    key: String,
    glyph: String,
    codepoint: Option<u32>,
    /// Keluarga warnanya. Ini bagian kontrak yang utama: pemanggil yang punya
    /// tema sendiri memetakan `family` ke gayanya.
    family: String,
    /// Warna bawaan `ticon`. Boleh diabaikan sepenuhnya.
    color: String,
    /// Petunjuk lebar sel glyph. `None` berarti tidak ada, dan konsumen
    /// sebaiknya memakai `width_default`.
    width: Option<usize>,
    /// Teks pengganti kalau glyph tidak bisa ditampilkan. `None` berarti tidak
    /// ada — lebih baik tidak ada daripada isinya tidak berbohong.
    fallback: Option<String>,
}

/// `--export`: seluruh aturan ke stdout, TSV atau JSON.
///
/// Keduanya dibangun dari daftar [`AturanEkspor`] yang sama, jadi isi ekspor
/// hanya ada di satu tempat.
fn print_export(rules: &Rules, glyphs: &Glyphs, format: Format) {
    let teks = match format {
        Format::Tsv => ekspor_tsv(rules, glyphs),
        Format::Json => ekspor_json(rules, glyphs),
    };
    print!("{teks}");
}

/// Bentuk TSV sebagai teks: datar, tanpa dependensi, enak dibaca `awk` dan
/// skrip shell. Kolomnya tidak berubah sejak 0.3.0 — JSON yang membawa skema.
///
/// Diekspos supaya bisa dipakai tanpa menjalankan perintah, dan supaya tes
/// bisa memeriksa isinya tanpa memproses biner.
pub fn ekspor_tsv(rules: &Rules, glyphs: &Glyphs) -> String {
    let mut out = String::from("jenis\tkunci\tglyph\tcodepoint\tkeluarga\twarna\n");
    for (keluarga, warna) in &rules.families {
        out.push_str(&format!("keluarga\t{keluarga}\t-\t-\t-\t{warna}\n"));
    }
    for a in aturan_ekspor(rules, glyphs) {
        let codepoint = a
            .codepoint
            .map(|c| format!("0x{c:x}"))
            .unwrap_or_else(|| "-".to_string());
        out.push_str(&format!(
            "{}\t{}\t{}\t{codepoint}\t{}\t{}\n",
            a.kind, a.key, a.glyph, a.family, a.color
        ));
    }
    out
}

fn aturan_ekspor(rules: &Rules, glyphs: &Glyphs) -> Vec<AturanEkspor> {
    let baris = |prioritas: Option<Prioritas>,
                 kind: &'static str,
                 key: &str,
                 glyph: &str,
                 family: &str,
                 width: Option<usize>,
                 fallback: Option<String>| AturanEkspor {
        prioritas,
        kind,
        key: key.to_string(),
        glyph: glyph.to_string(),
        codepoint: glyphs.get(glyph).map(|c| c as u32),
        family: family.to_string(),
        color: rules.color_of(family).to_string(),
        width,
        fallback,
    };

    let mut keluar = Vec::new();
    for category in rules.categories.values() {
        for ext in &category.ext {
            let glyph = category
                .by_ext
                .get(ext)
                .cloned()
                .unwrap_or_else(|| category.glyph.clone());
            keluar.push(baris(
                Some(Prioritas::Ekstensi),
                Prioritas::Ekstensi.kind(),
                ext,
                &glyph,
                &category.family,
                category.lebar,
                category.fallback.clone(),
            ));
        }
        for (nilai, prioritas) in category
            .names
            .iter()
            .map(|n| (n, Prioritas::Nama))
            .chain(category.prefix.iter().map(|n| (n, Prioritas::Awalan)))
            .chain(category.suffix.iter().map(|n| (n, Prioritas::Akhiran)))
        {
            keluar.push(baris(
                Some(prioritas),
                prioritas.kind(),
                nilai,
                &category.glyph,
                &category.family,
                category.lebar,
                category.fallback.clone(),
            ));
        }
    }
    for (nama, rule) in &rules.dirs {
        keluar.push(baris(
            None,
            "dir",
            nama,
            &rule.glyph,
            &rule.family,
            rule.lebar,
            rule.fallback.clone(),
        ));
    }
    keluar
}

/// Bentuk JSON sebagai teks: kontrak netral untuk konsumen non-Rust.
///
/// Yang membuatnya netral: urutan resolver ikut keluar sebagai angka
/// `priority` (jadi implementasi lain tidak perlu menebak urutan yang benar),
/// warna bawaan (`color`) tinggal satu field yang boleh diabaikan — yang
/// dipakai memetakan `family` ke gaya sendiri — dan `width` serta `fallback`
/// ikut dibawa **hanya kalau diisi**, supaya konsumen tanpa Nerd Font punya
/// pilihan tanpa dipaksa menebak.
///
/// Ditulis tangan supaya tidak menambah dependensi hanya untuk serialisasi.
pub fn ekspor_json(rules: &Rules, glyphs: &Glyphs) -> String {
    let aturan = aturan_ekspor(rules, glyphs);
    let mut out = String::new();
    let mut baris = |teks: &str| {
        out.push_str(teks);
        out.push('\n');
    };
    baris("{");
    baris(&format!("  \"$schema\": \"{}\",", SKEMA_URL));
    baris(&format!("  \"schema\": \"{}\",", peta::SKEMA));
    baris(&format!("  \"width_default\": {LEBAR_GLIF_BAWAAN},"));

    let urutan: Vec<String> = Prioritas::URUTAN
        .iter()
        .map(|p| json_teks(p.kind()))
        .collect();
    baris(&format!("  \"order\": [{}],", urutan.join(", ")));

    let keluarga: Vec<String> = rules
        .families
        .iter()
        .map(|(nama, warna)| format!("{}: {}", json_teks(nama), json_teks(warna)))
        .collect();
    baris(&format!("  \"families\": {{{}}},", keluarga.join(", ")));

    let bawaan = |kategori: &str| -> String {
        let Some(cat) = rules.categories.get(kategori) else {
            return "null".to_string();
        };
        let mut bagian = vec![
            format!("\"glyph\": {}", json_teks(&cat.glyph)),
            format!(
                "\"codepoint\": {}",
                glyphs
                    .get(&cat.glyph)
                    .map_or_else(|| "null".to_string(), |c| (c as u32).to_string())
            ),
            format!("\"family\": {}", json_teks(&cat.family)),
            format!("\"color\": {}", json_teks(rules.color_of(&cat.family))),
        ];
        if let Some(lebar) = cat.lebar {
            bagian.push(format!("\"width\": {lebar}"));
        }
        if let Some(ganti) = &cat.fallback {
            bagian.push(format!("\"fallback\": {}", json_teks(ganti)));
        }
        format!("{{{}}}", bagian.join(", "))
    };
    baris(&format!(
        "  \"defaults\": {{\"file\": {}, \"dir\": {}}},",
        bawaan(mapping::DEFAULT_FILE_CATEGORY),
        bawaan(mapping::DEFAULT_DIR_CATEGORY)
    ));

    baris("  \"rules\": [");
    let total = aturan.len();
    for (index, a) in aturan.iter().enumerate() {
        let mut bagian = vec![
            format!("\"kind\": {}", json_teks(a.kind)),
            format!(
                "\"priority\": {}",
                a.prioritas
                    .map_or_else(|| "null".to_string(), |p| p.angka().to_string())
            ),
            format!("\"key\": {}", json_teks(&a.key)),
            format!("\"family\": {}", json_teks(&a.family)),
            format!("\"color\": {}", json_teks(&a.color)),
            format!("\"glyph\": {}", json_teks(&a.glyph)),
            format!(
                "\"codepoint\": {}",
                a.codepoint
                    .map_or_else(|| "null".to_string(), |c| c.to_string())
            ),
        ];
        if let Some(lebar) = a.width {
            bagian.push(format!("\"width\": {lebar}"));
        }
        if let Some(ganti) = &a.fallback {
            bagian.push(format!("\"fallback\": {}", json_teks(ganti)));
        }
        let koma = if index + 1 < total { "," } else { "" };
        baris(&format!("    {{{}}}{koma}", bagian.join(", ")));
    }
    baris("  ]");
    baris("}");
    out
}

/// Bungkus `teks` sebagai string JSON, meloloskan karakter yang perlu.
fn json_teks(teks: &str) -> String {
    let mut out = String::with_capacity(teks.len() + 2);
    out.push('"');
    for c in teks.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Cetak daftar nilai dengan pembungkusan baris, menjorok di bawah labelnya.
fn wrapped(label: &str, values: &[String], width: usize) {
    if values.is_empty() {
        return;
    }

    let prefix = format!("    {label:<9} ");
    let continuation = " ".repeat(prefix.len());
    let available = width.saturating_sub(prefix.len()).max(24);

    let mut line = String::new();
    let mut first = true;
    for value in values {
        if line.is_empty() {
            line = value.clone();
        } else if UnicodeWidthStr::width(line.as_str()) + 1 + UnicodeWidthStr::width(value.as_str())
            <= available
        {
            line.push(' ');
            line.push_str(value);
        } else {
            let lead = if first { &prefix } else { &continuation };
            println!("{lead}{line}");
            first = false;
            line = value.clone();
        }
    }
    if !line.is_empty() {
        let lead = if first { &prefix } else { &continuation };
        println!("{lead}{line}");
    }
}

/// Padding berdasarkan lebar tampilan. `{:<w$}` menghitung char dan
/// `String::len` menghitung byte — keduanya salah untuk nama non-ASCII.
fn pad(text: &str, width: usize) -> String {
    let mut padded = String::from(text);
    padded.push_str(&" ".repeat(width.saturating_sub(UnicodeWidthStr::width(text))));
    padded
}

#[allow(clippy::too_many_arguments)]
fn print_category_row(
    rules: &Rules,
    glyphs: &Glyphs,
    options: &Options,
    name: &str,
    glyph_name: &str,
    color: &str,
    name_width: usize,
    glyph_width: usize,
) {
    let glyph = glyphs
        .get(glyph_name)
        .map(|character| character.to_string())
        .unwrap_or_else(|| "?".to_string());
    let icon = render::paint(rules, options.color, color, &glyph);
    let label = render::paint(rules, options.color, color, color);

    println!(
        "  {icon} {}  {glyph_name:<glyph_width$}  {label}",
        pad(name, name_width)
    );
}

/// `--list` dan `--gallery` bekerja pada pemetaan, bukan pada isi folder.
/// Path yang lewat sebelumnya diabaikan diam-diam; sekarang ditolak, karena
/// minta tabel sambil menyebut folder adalah harapan yang keliru.
fn cek_tanpa_path(flag: &str, options: &Options) -> Result<(), String> {
    match options.paths.first() {
        Some(path) => Err(format!(
            "`{flag}` tidak menerima path, tapi '{}' diberikan — tanpa path \
             ia bekerja pada seluruh tabel pemetaan",
            path.display()
        )),
        None => Ok(()),
    }
}

fn audit(rules: &Rules, glyphs: &Glyphs) -> Result<ExitCode, String> {
    let findings = rules.audit(glyphs);

    if findings.is_empty() {
        let extensions: usize = rules
            .categories
            .values()
            .map(|category| category.ext.len())
            .sum();
        let dim = rules.dim_rules();
        println!(
            "pemetaan bersih: {} kategori, {} folder khusus, {} ekstensi, {} glyph, {} keluarga warna",
            rules.categories.len(),
            rules.dirs.len(),
            extensions,
            glyphs.len(),
            rules.families.len()
        );
        println!("warna 'dim' terpakai {dim} aturan");
        return Ok(ExitCode::SUCCESS);
    }

    let mut report = String::new();
    let _ = writeln!(
        report,
        "ticon: {} masalah konsistensi di icons.toml:",
        findings.len()
    );
    for finding in &findings {
        let _ = writeln!(report, "  - {finding}");
    }
    Err(report.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Options {
        Options {
            paths: Vec::new(),
            all: true,
            one_per_line: true,
            icons: true,
            color: true,
            sort: Sort::Name,
            width: None,
            icons_map: None,
            icons_pack: None,
        }
    }

    /// Tanpa escape warna milik `paint()` — untuk memeriksa isi data apa adanya.
    fn options_plain() -> Options {
        Options {
            color: false,
            ..options()
        }
    }

    /// Injeksi terminal hanya bisa dibuktikan di tempat komposisinya: nama
    /// berkas dari luar tidak boleh keluar mentah, baik RLO maupun ESC, dan
    /// lebar kolom harus diukur dari teks yang sudah disanitasi.
    ///
    /// Warna dimatikan di sini: `paint()` memang menulis escape miliknya
    /// sendiri, jadi yang diperiksa adalah isi datanya, bukan string ansinya.
    ///
    /// Uji e2e di `tests/e2e.rs` menutup kasus RLO lewat berkas sungguhan;
    /// ESC tidak bisa dibuat sebagai nama berkas di Windows, jadi diuji di sini.
    #[test]
    fn nama_berkarakter_kontrol_disanitasi_saat_dicetak() {
        let rules = Rules::load().expect("icons.toml harus bisa dibaca");
        let glyphs = Glyphs::bundled();
        let entry = Entry {
            name: "invoice\u{202e}fdp.exe\u{1b}[2J".to_string(),
            is_dir: false,
            link_target: None,
            size: 0,
            modified: None,
        };
        let opsi = options_plain();

        let item = render_entry(&rules, &glyphs, &opsi, &entry, &Bentuk::kosong());

        assert!(
            !item.painted.contains('\u{1b}') && !item.painted.contains('\u{202e}'),
            "karakter kontrol tidak boleh keluar mentah: {:?}",
            item.painted
        );
        assert!(
            item.painted.contains("\\u{202e}") && item.painted.contains("\\u{1b}"),
            "harus muncul sebagai representasi aman: {:?}",
            item.painted
        );
        let nama_aman = render::sanitize(&entry.name);
        let lebar = UnicodeWidthStr::width(nama_aman.as_ref());
        assert_eq!(item.width, 2 + lebar, "lebar diukur dari teks aman");
    }

    /// Target symlink juga berasal dari luar, jadi ikut disanitasi.
    #[test]
    fn target_symlink_karakter_kontrol_disanitasi() {
        let rules = Rules::load().expect("icons.toml harus bisa dibaca");
        let glyphs = Glyphs::bundled();
        let entry = Entry {
            name: "tautan".to_string(),
            is_dir: false,
            link_target: Some("target\u{1b}[2Jevil".to_string()),
            size: 0,
            modified: None,
        };
        let opsi = options_plain();

        let item = render_entry(&rules, &glyphs, &opsi, &entry, &Bentuk::kosong());

        assert!(!item.painted.contains('\u{1b}'), "{:?}", item.painted);
        assert!(item.painted.contains("\\u{1b}"), "{:?}", item.painted);
    }

    #[test]
    fn list_dan_gallery_menolak_path() {
        let mut opsi = options();
        opsi.paths = vec![PathBuf::from("src")];
        assert!(cek_tanpa_path("--list", &opsi).is_err());
        assert!(cek_tanpa_path("--gallery", &opsi).is_err());
        let pesan = cek_tanpa_path("--list", &opsi).unwrap_err();
        assert!(pesan.contains("--list") && pesan.contains("src"), "{pesan}");

        opsi.paths.clear();
        assert!(cek_tanpa_path("--list", &opsi).is_ok());
    }
}
