//! `ikon` — ikon minimalis dan flat untuk terminal.
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

pub mod cli;
pub mod glyph;
pub mod mapping;
pub mod render;
pub mod terminal;

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

use cli::{Command, Options, Sort};
use glyph::Glyphs;
use mapping::Rules;
use render::Item;

pub fn run() -> ExitCode {
    match execute() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("ikon: {message}");
            ExitCode::FAILURE
        }
    }
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
            println!("ikon {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        Command::Error(message) => Err(format!("{message}\n\nCoba `ikon --help`")),
        Command::Audit => {
            let rules = Rules::load()?;
            let glyphs = Glyphs::bundled();
            audit(&rules, &glyphs)
        }
        Command::Mapping(options) => {
            let rules = Rules::load()?;
            let glyphs = Glyphs::bundled();
            print_mapping(&rules, &glyphs, &options);
            Ok(ExitCode::SUCCESS)
        }
        Command::Gallery(options) => {
            let rules = Rules::load()?;
            let glyphs = Glyphs::bundled();
            print_gallery(&rules, &glyphs, &options);
            Ok(ExitCode::SUCCESS)
        }
        Command::Dir(options) => {
            let rules = Rules::load()?;
            let glyphs = Glyphs::bundled();
            list(&rules, &glyphs, &options)
        }
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

fn list(rules: &Rules, glyphs: &Glyphs, options: &Options) -> Result<ExitCode, String> {
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
                eprintln!("ikon: {}: {error}", path.display());
                failures += 1;
                continue;
            }
        };

        if show_headers {
            if index > 0 {
                let _ = writeln!(out);
            }
            let header = format!("{}:", path.display());
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
                .map(|entry| render_entry(rules, glyphs, options, entry))
                .collect()
        } else {
            vec![render_entry(
                rules,
                glyphs,
                options,
                &Entry::from_path(path, &metadata),
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

fn render_entry(rules: &Rules, glyphs: &Glyphs, options: &Options, entry: &Entry) -> Item {
    let resolved = if entry.is_dir {
        rules.resolve_dir(&entry.name)
    } else {
        rules.resolve_file(&entry.name)
    };

    let mut painted = String::new();
    let mut plain = String::new();

    if options.icons {
        if let Some(glyph) = glyphs.get(&resolved.glyph) {
            let cell = format!("{glyph} ");
            plain.push_str(&cell);
            painted.push_str(&render::paint(rules, options.color, &resolved.color, &cell));
        }
    }

    // Hanya ikon yang diberi warna. Nama dibiarkan memakai warna teks bawaan
    // terminal supaya daftarnya tetap tenang — penanda jenis berkas sudah
    // dibawa oleh warna ikon.
    plain.push_str(&entry.name);
    match entry.link_target {
        Some(_) => painted.push_str(&render::paint(rules, options.color, "cyan", &entry.name)),
        None => painted.push_str(&entry.name),
    }

    if let Some(target) = &entry.link_target {
        let suffix = format!(" → {target}");
        plain.push_str(&suffix);
        painted.push_str(&render::paint(rules, options.color, "dim", &suffix));
    }

    Item::styled(painted, &plain)
}

// ---------------------------------------------------------------------------
//  Tabel pemetaan dan audit
// ---------------------------------------------------------------------------

/// Galeri contoh: satu baris per aturan, dan contoh namanya benar-benar
/// dilewatkan ke resolver yang sama dengan yang dipakai `ikon` sehari-hari.
/// Jadi yang kamu lihat di sini bukan gambar atas nama desain — itu memang
/// hasil yang akan keluar.
fn print_gallery(rules: &Rules, glyphs: &Glyphs, options: &Options) {
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

    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    for (example, label, is_dir) in examples {
        let resolved = if is_dir {
            rules.resolve_dir(&example)
        } else {
            rules.resolve_file(&example)
        };
        if resolved.glyph.is_empty() {
            continue;
        }
        rows.push((example, label, resolved.glyph, resolved.color));
    }

    let example_width = rows.iter().map(|row| row.0.len()).max().unwrap_or(0);
    let label_width = rows.iter().map(|row| row.1.len()).max().unwrap_or(0);

    println!(
        "galeri ikon ({} aturan, {} glyph tertanam):",
        rows.len(),
        glyphs.len()
    );
    println!();
    for (example, label, glyph_name, color) in &rows {
        let character = glyphs
            .get(glyph_name)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "?".to_string());
        let icon = render::paint(rules, options.color, color, &character);
        let name = render::paint(rules, options.color, color, example);
        let swatch = render::paint(rules, options.color, color, color);
        println!("  {icon}  {name:<example_width$}  {label:<label_width$}  {swatch}");
    }
}

/// Tampilan untuk meninjau desain: satu baris per kategori, lalu rincian
/// aturannya menjorok di bawahnya. Sengaja dijaga agar tetap bisa di-grep —
/// `ikon --list | grep language_go` harus tetap berguna.
fn print_mapping(rules: &Rules, glyphs: &Glyphs, options: &Options) {
    let name_width = rules.categories.keys().map(String::len).max().unwrap_or(0);
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
        } else if line.len() + 1 + value.len() <= available {
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

    println!("  {icon} {name:<name_width$}  {glyph_name:<glyph_width$}  {label}");
}

fn audit(rules: &Rules, glyphs: &Glyphs) -> Result<ExitCode, String> {
    let findings = rules.audit(glyphs);

    if findings.is_empty() {
        let extensions: usize = rules
            .categories
            .values()
            .map(|category| category.ext.len())
            .sum();
        let dim = rules
            .categories
            .values()
            .filter(|category| rules.color_of(&category.family) == "dim")
            .count()
            + rules
                .dirs
                .values()
                .filter(|rule| rules.color_of(&rule.family) == "dim")
                .count();
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
        "ikon: {} masalah konsistensi di icons.toml:",
        findings.len()
    );
    for finding in &findings {
        let _ = writeln!(report, "  - {finding}");
    }
    Err(report.trim_end().to_string())
}
