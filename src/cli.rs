//! Parsing argumen.
//!
//! Ditulis tangan, bukan lewat crate CLI, karena permukaannya memang kecil —
//! dan karena `ticon` seharusnya tidak punya lebih banyak opsi daripada yang
//! bisa dijelaskan dalam satu layar.

use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;

/// Teks bantuan yang dicetak `ticon --help`.
pub const HELP: &str = "\
ticon — ikon minimalis dan flat untuk terminal

Pakai:
    ticon [opsi] [path...]

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
        --explain <nama>   kenapa nama itu dapat ikon tersebut
        --export           cetak semua aturan sebagai tabel TSV
    -h, --help             tampilkan bantuan ini
    -V, --version          tampilkan versi

Lingkungan:
    NO_COLOR           matikan warna kalau diisi
    CLICOLOR_FORCE     paksa warna kalau bukan \"0\"
    TICON_ICONS         nilai bawaan untuk --icons
    TICON_COLOR         nilai bawaan untuk --color
    COLUMNS            lebar terminal kalau tidak bisa dideteksi

Contoh:
    ticon
    ticon -a --sort size
    ticon --list | grep cyan
    ticon --audit

Ikon memakai glyph Nerd Fonts, jadi font terminal kamu perlu versi yang sudah
di-patch. Warna memakai 16 warna ANSI terminal, jadi paletnya ikut berubah
saat kamu ganti theme terminal.

Kalau terminalnya tidak terdeteksi sebagai terminal (misalnya MinTTY di Git
Bash), ikon dan warna dimatikan otomatis. Pakai `--icons always`, atau set
`TICON_ICONS=always` supaya selalu tampil. `IKON_ICONS` dan `IKON_COLOR`
masih dibaca sebagai alias.
";

/// Kunci pengurutan entri direktori.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// Berdasarkan nama (huruf besar/kecil diabaikan).
    Name,
    /// Berdasarkan ekstensi, lalu nama.
    Ext,
    /// Berdasarkan ukuran, terbesar dulu.
    Size,
    /// Berdasarkan waktu modifikasi, terbaru dulu.
    Time,
}

/// Opsi yang sudah ternormalisasi: tidak ada lagi mode `auto`/lingkungan yang
/// belum diputuskan — semua sudah dijawab di [`parse`].
#[derive(Debug)]
pub struct Options {
    /// Path yang diminta; kosong berarti direktori saat ini.
    pub paths: Vec<PathBuf>,
    /// Tampilkan berkas tersembunyi (`-a`).
    pub all: bool,
    /// Satu entri per baris (`-1`), bukan grid kolom.
    pub one_per_line: bool,
    /// Tampilkan glyph ikon.
    pub icons: bool,
    /// Gunakan warna ANSI.
    pub color: bool,
    /// Kunci pengurutan entri.
    pub sort: Sort,
    /// Lebar kolom yang dipaksakan; `None` = pakai lebar terminal.
    pub width: Option<usize>,
}

/// Perintah yang bisa diminta ke `ticon`, hasil parsing argumen.
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
    /// Jelaskan kenapa sebuah nama mendapat ikon tertentu.
    Explain(String),
    /// Cetak seluruh aturan sebagai tabel TSV ke stdout.
    Export,
    /// Cetak bantuan dan keluar.
    Help,
    /// Cetak versi dan keluar.
    Version,
    /// Argumen tidak bisa dipakai; isinya pesan untuk pengguna.
    Error(String),
}

/// Tiga tingkat untuk opsi yang bisa `auto`: tentukan sendiri, paksa, atau
/// jangan pernah.
enum Mode {
    /// Biarkan `ticon` yang memutuskan (dari tty, lingkungan, dan tema).
    Auto,
    /// Selalu nyalakan, apa pun kondisi terminal.
    Always,
    /// Selalu matikan.
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

/// Nilai lingkungan yang salah tidak lagi ditelan diam-diam — sama seperti
/// `--icons=<nilai>` yang keluar dengan pesan error.
/// Nilai dari variabel lingkungan untuk satu opsi. `TICON_*` adalah nama
/// (resmi; `IKON_*` masih diterima sebagai alias supaya konfigurasi lama tidak
/// langsung mati. Nilai yang tidak dikenal **tidak** ditelan diam-diam:
/// kesalahannya dikembalikan sebagai pesan, sama seperti `--icons=<nilai>`.
fn env_mode(nama_resmi: &str) -> Result<Option<Mode>, String> {
    let alias = format!("IKON_{}", nama_resmi.trim_start_matches("TICON_"));
    let (terpakai, nilai) = match env::var(nama_resmi) {
        Ok(nilai) => (nama_resmi, Some(nilai)),
        Err(_) => (alias.as_str(), env::var(&alias).ok()),
    };
    let Some(nilai) = nilai else {
        return Ok(None);
    };
    Mode::parse(&nilai)
        .map(Some)
        .map_err(|error| format!("{terpakai}: {error}"))
}

/// Ubah argumen menjadi [`Command`].
///
/// Tidak pernah gagal dengan panic atau `Result`: kesalahan argumen dikembalikan
/// sebagai [`Command::Error`] yang pesannya siap ditampilkan, sehingga pemanggil
/// (dan tes) tidak perlu memproses kesalahan sendiri.
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
            "--explain" => match take_value("--explain").and_then(|value| {
                if value.is_empty() {
                    Err("opsi --explain butuh nama".to_string())
                } else {
                    Ok(value)
                }
            }) {
                Ok(name) => return Command::Explain(name),
                Err(message) => return Command::Error(message),
            },
            "--export" => return Command::Export,
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

    let icon_mode = match icon_mode {
        Some(mode) => mode,
        None => match env_mode("TICON_ICONS") {
            Ok(mode) => mode.unwrap_or(Mode::Auto),
            Err(error) => return Command::Error(error),
        },
    };
    let color_mode = match color_mode {
        Some(mode) => mode,
        None => match env_mode("TICON_COLOR") {
            Ok(mode) => mode.unwrap_or(Mode::Auto),
            Err(error) => return Command::Error(error),
        },
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `parse()` membaca TICON_ICONS/TICON_COLOR/NO_COLOR/CLICOLOR_FORCE di setiap
    // pemanggilan, jadi semua tes mengunci satu mutex yang sama: tes yang
    // mengubah lingkungan tidak boleh berlomba dengan tes lain.
    static ENV: Mutex<()> = Mutex::new(());

    fn parse_dalam_kunci(args: &[&str]) -> Command {
        let _guard = ENV.lock().unwrap_or_else(|racun| racun.into_inner());
        parse(args.iter().map(|arg| arg.to_string()))
    }

    fn opsi(command: &Command) -> &Options {
        match command {
            Command::Dir(options) | Command::Mapping(options) | Command::Gallery(options) => {
                options
            }
            lain => panic!("bukan perintah daftar-berkas: {lain:?}"),
        }
    }

    #[test]
    fn jalur_posisi_dan_bendera_pendek() {
        let command = parse_dalam_kunci(&["-a", "-1", "src", "."]);
        let opsi = opsi(&command);
        assert!(opsi.all && opsi.one_per_line);
        assert_eq!(opsi.paths, vec![PathBuf::from("src"), PathBuf::from(".")]);
    }

    #[test]
    fn nilai_bisa_inline_atau_argumen_berikutnya() {
        let command = parse_dalam_kunci(&["--width=40", "--sort", "ext", "--color", "never"]);
        let opsi = opsi(&command);
        assert_eq!(opsi.width, Some(40));
        assert_eq!(opsi.sort, Sort::Ext);
        assert!(!opsi.color);
    }

    #[test]
    fn dua_pemisah_menghentikan_parsing_opsi() {
        let command = parse_dalam_kunci(&["--", "-a", "--width"]);
        let opsi = opsi(&command);
        assert!(!opsi.all, "-a setelah -- adalah jalur, bukan opsi");
        assert_eq!(opsi.width, None);
        assert_eq!(opsi.paths.len(), 2);
    }

    #[test]
    fn opsi_atau_mode_tidak_dikenal_menghasilkan_error() {
        assert!(matches!(
            parse_dalam_kunci(&["--ngawur"]),
            Command::Error(_)
        ));
        let Command::Error(pesan) = parse_dalam_kunci(&["--color", "tubeh"]) else {
            panic!("mode tak dikenal harus jadi Command::Error");
        };
        assert!(pesan.contains("tidak dikenal"), "{pesan}");
    }

    #[test]
    fn lingkungan_menimpa_bawaan_dan_menolak_nilai_salah() {
        // Memakai `parse` langsung: mutex-nya sudah dipegang di sini.
        let _guard = ENV.lock().unwrap_or_else(|racun| racun.into_inner());

        env::set_var("TICON_ICONS", "always");
        let command = parse(std::iter::empty());
        assert!(opsi(&command).icons);
        env::remove_var("TICON_ICONS");

        env::set_var("TICON_COLOR", "bogus");
        let Command::Error(pesan) = parse(std::iter::empty()) else {
            panic!("TICON_COLOR tak dikenal harus jadi Command::Error");
        };
        assert!(pesan.contains("TICON_COLOR"), "{pesan}");
        env::remove_var("TICON_COLOR");
    }

    /// `IKON_*` masih dibaca sebagai alias supaya konfigurasi lama dari versi
    /// sebelum rename tidak langsung mati. Nama resmi tetap menang kalau dua-duanya
    /// diset.
    #[test]
    fn alias_lama_ikon_masih_dibaca() {
        let _guard = ENV.lock().unwrap_or_else(|racun| racun.into_inner());

        env::set_var("IKON_ICONS", "always");
        let command = parse(std::iter::empty());
        assert!(opsi(&command).icons, "alias IKON_ICONS harus berlaku");
        env::remove_var("IKON_ICONS");

        // Nama resmi menang ketika keduanya ada.
        env::set_var("IKON_ICONS", "always");
        env::set_var("TICON_ICONS", "never");
        let command = parse(std::iter::empty());
        assert!(!opsi(&command).icons, "TICON_ICONS harus menang atas alias");
        env::remove_var("IKON_ICONS");
        env::remove_var("TICON_ICONS");
    }
}
