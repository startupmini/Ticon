//! Parsing argumen.
//!
//! Ditulis tangan, bukan lewat crate CLI, karena permukaannya memang kecil —
//! dan karena `ikon` seharusnya tidak punya lebih banyak opsi daripada yang
//! bisa dijelaskan dalam satu layar.

use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;

pub const HELP: &str = "\
ikon — ikon minimalis dan flat untuk terminal

Pakai:
    ikon [opsi] [path...]

Opsi:
    -a, --all              tampilkan berkas tersembunyi
    -1                     satu entri per baris
        --icons <mode>     auto | always | never   (bawaan: auto)
        --color <mode>     auto | always | never   (bawaan: auto)
        --sort <kunci>     name | ext | size | time (bawaan: name)
        --width <kolom>    paksa lebar kolom untuk tata letak
        --list             cetak tabel pemetaan ikon
        --gallery          cetak contoh ikon dari tiap aturan
        --audit            periksa konsistensi pemetaan
    -h, --help             tampilkan bantuan ini
    -V, --version          tampilkan versi

Lingkungan:
    NO_COLOR           matikan warna kalau diisi
    CLICOLOR_FORCE     paksa warna kalau bukan \"0\"
    IKON_ICONS         nilai bawaan untuk --icons
    IKON_COLOR         nilai bawaan untuk --color
    COLUMNS            lebar terminal kalau tidak bisa dideteksi

Contoh:
    ikon
    ikon -a --sort size
    ikon --list | grep cyan
    ikon --audit

Ikon memakai glyph Nerd Fonts, jadi font terminal kamu perlu versi yang sudah
di-patch. Warna memakai 16 warna ANSI terminal, jadi paletnya ikut berubah
saat kamu ganti theme terminal.

Kalau terminalnya tidak terdeteksi sebagai terminal (misalnya MinTTY di Git
Bash), ikon dan warna dimatikan otomatis. Pakai `--icons always`, atau set
`IKON_ICONS=always` supaya selalu tampil.
";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Name,
    Ext,
    Size,
    Time,
}

#[derive(Debug)]
pub struct Options {
    pub paths: Vec<PathBuf>,
    pub all: bool,
    pub one_per_line: bool,
    pub icons: bool,
    pub color: bool,
    pub sort: Sort,
    pub width: Option<usize>,
}

#[derive(Debug)]
pub enum Command {
    /// Tampilkan isi direktori.
    Dir(Options),
    /// Cetak tabel pemetaan ikon.
    Mapping(Options),
    /// Cetak contoh hasil ikon, satu per aturan.
    Gallery(Options),
    /// Periksa konsistensi pemetaan.
    Audit,
    Help,
    Version,
    Error(String),
}

enum Mode {
    Auto,
    Always,
    Never,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" | "on" | "yes" => Ok(Self::Always),
            "never" | "off" | "no" => Ok(Self::Never),
            other => Err(format!(
                "mode '{other}' tidak dikenal (auto, always, never)"
            )),
        }
    }
}

fn env_mode(name: &str) -> Option<Mode> {
    env::var(name)
        .ok()
        .and_then(|value| Mode::parse(&value).ok())
}

pub fn parse(args: impl Iterator<Item = String>) -> Command {
    let mut options = Options {
        paths: Vec::new(),
        all: false,
        one_per_line: false,
        icons: true,
        color: true,
        sort: Sort::Name,
        width: None,
    };

    // `None` berarti belum ditentukan; nilai bawaan diambil dari lingkungan
    // lebih dulu, baru dari deteksi terminal.
    let mut icon_mode: Option<Mode> = None;
    let mut color_mode: Option<Mode> = None;
    let mut positional_only = false;
    let mut show_mapping = false;
    let mut show_gallery = false;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        if positional_only || !arg.starts_with('-') || arg == "-" {
            options.paths.push(PathBuf::from(arg));
            continue;
        }

        let (flag, inline_value) = match arg.split_once('=') {
            Some((flag, value)) => (flag.to_string(), Some(value.to_string())),
            None => (arg.clone(), None),
        };

        // Ambil nilai dari `--flag=nilai` atau dari argumen berikutnya.
        let mut take_value = |flag: &str| -> Result<String, String> {
            match &inline_value {
                Some(value) => Ok(value.clone()),
                None => args
                    .next()
                    .ok_or_else(|| format!("opsi {flag} butuh nilai")),
            }
        };

        let outcome: Result<(), String> = match flag.as_str() {
            "--" => {
                positional_only = true;
                Ok(())
            }
            "-h" | "--help" => return Command::Help,
            "-V" | "--version" => return Command::Version,
            "-a" | "--all" => {
                options.all = true;
                Ok(())
            }
            "-1" => {
                options.one_per_line = true;
                Ok(())
            }
            "--list" => {
                show_mapping = true;
                Ok(())
            }
            "--gallery" => {
                show_gallery = true;
                Ok(())
            }
            "--audit" => return Command::Audit,
            "--icons" => take_value("--icons").and_then(|value| {
                icon_mode = Some(Mode::parse(&value)?);
                Ok(())
            }),
            "--color" | "--colour" => take_value("--color").and_then(|value| {
                color_mode = Some(Mode::parse(&value)?);
                Ok(())
            }),
            "--no-color" => {
                color_mode = Some(Mode::Never);
                Ok(())
            }
            "--width" => take_value("--width").and_then(|value| {
                let columns: usize = value
                    .parse()
                    .map_err(|_| format!("lebar '{value}' bukan angka"))?;
                options.width = (columns > 0).then_some(columns);
                Ok(())
            }),
            "--sort" => take_value("--sort").and_then(|value| {
                options.sort = match value.as_str() {
                    "name" => Sort::Name,
                    "ext" | "extension" => Sort::Ext,
                    "size" => Sort::Size,
                    "time" | "mtime" => Sort::Time,
                    other => return Err(format!("kunci urut '{other}' tidak dikenal")),
                };
                Ok(())
            }),
            other => Err(format!("opsi '{other}' tidak dikenal")),
        };

        if let Err(message) = outcome {
            return Command::Error(message);
        }
    }

    let icon_mode = icon_mode
        .or_else(|| env_mode("IKON_ICONS"))
        .unwrap_or(Mode::Auto);
    let color_mode = color_mode
        .or_else(|| env_mode("IKON_COLOR"))
        .unwrap_or(Mode::Auto);

    let stdout_is_tty = std::io::stdout().is_terminal();
    let no_color_env = env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    let force_color_env = env::var_os("CLICOLOR_FORCE").is_some_and(|value| value != "0");

    options.color = match color_mode {
        Mode::Always => true,
        Mode::Never => false,
        Mode::Auto => !no_color_env && (stdout_is_tty || force_color_env),
    };
    options.icons = match icon_mode {
        Mode::Always => true,
        Mode::Never => false,
        Mode::Auto => stdout_is_tty || force_color_env,
    };

    if show_mapping {
        Command::Mapping(options)
    } else if show_gallery {
        Command::Gallery(options)
    } else {
        Command::Dir(options)
    }
}
