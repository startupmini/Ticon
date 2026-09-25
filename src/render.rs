//! Pewarnaan dan tata letak.
//!
//! Yang dirender selalu berupa pasangan: teks yang sudah diwarnai untuk
//! ditulis ke layar, dan lebar tampilan teks polosnya. ANSI escape tidak
//! dihitung sebagai lebar, jadi keduanya dipisah sejak awal — bukan dengan
//! menghitung ulang setelah string diwarnai.

use std::fmt::Write as _;

use unicode_width::UnicodeWidthStr;

use crate::mapping::Rules;

/// Jarak antar kolom. Cukup untuk ikon + satu spasi tanpa terlihat rapat.
pub const GAP: usize = 2;

pub fn paint(rules: &Rules, enabled: bool, color: &str, text: &str) -> String {
    if !enabled {
        return text.to_string();
    }
    match rules.ansi(color) {
        Some(code) => format!("\x1b[{code}m{text}\x1b[0m"),
        None => text.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub painted: String,
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
fn best_column_count(items: &[Item], term_width: usize) -> usize {
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
}
