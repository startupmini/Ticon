//! Pewarnaan dan tata letak.
//!
//! Yang dirender selalu berupa pasangan: teks yang sudah diwarnai untuk
//! ditulis ke layar, dan lebar tampilan teks polosnya. ANSI escape tidak
//! dihitung sebagai lebar, jadi keduanya dipisah sejak awal — bukan dengan
//! menghitung ulang setelah string diwarnai.

use std::borrow::Cow;
use std::fmt::Write as _;

use unicode_width::UnicodeWidthStr;

use crate::mapping::Rules;

/// Jarak antar kolom. Cukup untuk ikon + satu spasi tanpa terlihat rapat.
pub const GAP: usize = 2;

/// Bungkus `text` dengan escape ANSI untuk warna `color`, atau kembalikan
/// apa adanya kalau warna dimatikan atau nama warnanya tidak dikenal.
pub fn paint(rules: &Rules, enabled: bool, color: &str, text: &str) -> String {
    if !enabled {
        return text.to_string();
    }
    match rules.ansi(color) {
        Some(code) => format!("\x1b[{code}m{text}\x1b[0m"),
        None => text.to_string(),
    }
}

/// Satu karakter yang tidak boleh sampai ke terminal: kontrol C0 (termasuk
/// ESC yang dieksekusi terminal), DEL/C1, pemisah baris Unicode, dan kontrol
/// bidi yang bisa membalik tampilan nama — serangan klasik
/// `invoice<RLO>fdp.exe` atau `nama\u{1b}[2J`.
fn berbahaya(c: char) -> bool {
    matches!(
        c,
        '\u{0}'..='\u{1f}'   // C0: ESC, BEL, newline, tab, ...
            | '\u{7f}'..='\u{9f}' // DEL + C1 (CSI 8-bit termasuk)
            | '\u{61c}'       // ALM
            | '\u{2028}'      // line/paragraph separator = pemutus baris
            | '\u{2029}'
            | '\u{202a}'..='\u{202e}' // LRE RLE PDF LRO RLO — pengendali arah
            | '\u{2066}'..='\u{2069}' // LRI RLI FSI PDI — isolasi bidi
    )
}

/// Apakah aman mencetak `text` apa adanya.
pub fn is_terminal_safe(text: &str) -> bool {
    !text.chars().any(berbahaya)
}

/// Apakah satu karakter aman dicetak.
pub fn is_safe_char(c: char) -> bool {
    !berbahaya(c)
}

/// Ganti karakter berbahaya dengan representasi ASCII terlihat (`\u{1b}`).
/// Nama tetap terbaca untuk debugging, tapi terminal tidak mengeksekusinya —
/// dan lebar tampilannya ASCII, jadi kolom daftar tetap lurus.
pub fn sanitize(text: &str) -> Cow<'_, str> {
    if is_terminal_safe(text) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if berbahaya(c) {
            let _ = write!(out, "\\u{{{:x}}}", c as u32);
        } else {
            out.push(c);
        }
    }
    Cow::Owned(out)
}

/// Satu entri siap ditulis: teks yang sudah berwarna untuk layar, dan lebar
/// tampilannya yang dihitung dari teks polos.
#[derive(Debug, Clone)]
pub struct Item {
    /// Teks dengan escape ANSI; inilah yang ditulis ke stdout.
    pub painted: String,
    /// Lebar tampilan dalam sel, diukur tanpa escape.
    pub width: usize,
}

impl Item {
    /// `painted` untuk layar, `plain` untuk mengukur lebar.
    pub fn styled(painted: String, plain: &str) -> Self {
        Self {
            painted,
            width: UnicodeWidthStr::width(plain),
        }
    }
}

/// Susun entri secara kolom-mayor (seperti `ls -C`), lalu tulis per baris.
pub fn grid(items: &[Item], term_width: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    let count = items.len();
    let columns = best_column_count(items, term_width);
    let rows = count.div_ceil(columns);

    let mut widths = vec![0usize; columns];
    for (index, item) in items.iter().enumerate() {
        let column = index / rows;
        widths[column] = widths[column].max(item.width);
    }

    let mut out = String::new();
    for row in 0..rows {
        let mut line = String::new();
        for (column, column_width) in widths.iter().enumerate() {
            let index = column * rows + row;
            if index >= count {
                break;
            }
            let item = &items[index];
            line.push_str(&item.painted);

            // Kolom terakhir di baris tidak perlu padding.
            let last_in_row = index + rows >= count;
            if !last_in_row {
                let padding = column_width.saturating_sub(item.width) + GAP;
                line.extend(std::iter::repeat_n(' ', padding));
            }
        }
        let _ = writeln!(out, "{}", line.trim_end());
    }
    out
}

/// Kolom terbanyak yang masih muat. Dicoba dari yang paling lebar supaya
/// hasilnya sepadat mungkin, tapi tidak pernah melewati lebar terminal.
///
/// Lebar per kolom hanya dihitung ulang saat `rows` berubah: nilai
/// `rows = ceil(count/columns)` cuma punya O(√count) kemungkinan, jadi total
/// O(n√n). Menghitung ulang untuk tiap kandidat (dulu O(n²)) membuat daftar
/// 16 ribu berkas berputar >7 detik — biaya yang bisa dipaksa penanam file.
fn best_column_count(items: &[Item], term_width: usize) -> usize {
    let count = items.len();
    let mut rows_kini = 0usize;
    // `prefix[j]` = jumlah lebar kolom 0..j untuk `rows_kini`. Kandidat
    // `columns` berjalan menurun, jadi `prefix[columns]` selalu terjangkau.
    let mut prefix: Vec<usize> = Vec::new();

    for columns in (1..=count).rev() {
        let rows = count.div_ceil(columns);
        if rows != rows_kini {
            rows_kini = rows;
            let mut widths = vec![0usize; columns];
            for (index, item) in items.iter().enumerate() {
                let column = index / rows;
                if column < columns {
                    widths[column] = widths[column].max(item.width);
                }
            }
            prefix.clear();
            prefix.push(0);
            for width in widths {
                let akhir = prefix[prefix.len() - 1] + width;
                prefix.push(akhir);
            }
        }
        if prefix[columns] + GAP * (columns - 1) <= term_width {
            return columns;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(labels: &[&str]) -> Vec<Item> {
        labels
            .iter()
            .map(|label| Item::styled(label.to_string(), label))
            .collect()
    }

    #[test]
    fn satu_entri_per_baris_kalau_sempit() {
        let items = items(&["alpha", "beta"]);
        let rendered = grid(&items, 5);
        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn entri_muat_dalam_satu_baris_kalau_lebar() {
        let items = items(&["alpha", "beta"]);
        let rendered = grid(&items, 80);
        assert_eq!(rendered.lines().count(), 1);
        assert_eq!(rendered.trim_end(), "alpha  beta");
    }

    #[test]
    fn tidak_ada_spasi_ekor_di_akhir_baris() {
        let items = items(&["a", "bbbbbbbb"]);
        let rendered = grid(&items, 4);
        for line in rendered.lines() {
            assert_eq!(line, line.trim_end());
        }
    }

    #[test]
    fn lebar_dihitung_tanpa_ansi() {
        let painted = "\u{1b}[31mikon\u{1b}[0m";
        let item = Item::styled(painted.to_string(), "ikon");
        assert_eq!(item.width, 4);
    }

    /// Bukti terhadap injeksi terminal: karakter kontrol dan pengendali bidi
    /// keluar sebagai representasi ASCII yang terlihat, bukan dieksekusi.
    #[test]
    fn karakter_kontrol_diganti_representasi_aman() {
        let kotor = "invoice\u{202e}fdp.exe\u{1b}[2J";
        let bersih = sanitize(kotor);
        assert_eq!(bersih, r"invoice\u{202e}fdp.exe\u{1b}[2J");
        assert!(is_terminal_safe(&bersih), "hasil sanitize harus aman");

        // Teks normal tidak disentuh — Cow::Borrowed, tanpa alokasi.
        assert_eq!(sanitize("readme.md"), "readme.md");
        assert!(is_terminal_safe("README.md"));
    }

    /// Referensi kuadratik yang asli. Versi cepat wajib menghasilkan keputusan
    /// yang sama persis untuk semua lebar — fungsi ini sengaja dipertahankan
    /// di tes sebagai jaring regresi.
    fn referensi_jumlah_kolom(items: &[Item], term_width: usize) -> usize {
        let count = items.len();
        for columns in (1..=count).rev() {
            let rows = count.div_ceil(columns);
            let mut total = GAP * (columns - 1);
            for column in 0..columns {
                let mut width = 0;
                for row in 0..rows {
                    let index = column * rows + row;
                    if index < count {
                        width = width.max(items[index].width);
                    }
                }
                total += width;
            }
            if total <= term_width {
                return columns;
            }
        }
        1
    }

    #[test]
    fn jumlah_kolom_identik_dengan_referensi() {
        let labels = [
            "a",
            "bb",
            "ccc",
            "dddd",
            "e",
            "ffffffffffffffff",
            "gg",
            "hhhhhhhhh",
            "iiii",
            "j",
            "kkkk",
            "lllllllllllll",
            "mmm",
            "nnnnnn",
            "o",
        ];
        let items = items(&labels);
        for width in 1..=90 {
            assert_eq!(
                best_column_count(&items, width),
                referensi_jumlah_kolom(&items, width),
                "jumlah kolom berbeda dari referensi pada lebar {width}"
            );
        }
    }
}
