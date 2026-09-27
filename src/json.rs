//! Parser JSON minimal, tanpa dependensi.
//!
//! Cukup untuk membaca peta ikon `ticon-map/2`, jadi sengaja sempit: menolak
//! apa pun yang di luar catenary-nya, bukan "meloloskan saja". Kalau peta ini
//! dibaca dari berkas yang tidak dipercaya, parser longgar bisa jadi celah;
//! parser ketat tidak.
//!
//! Yang ditolak: sisa karakter setelah nilai terakhir, literal yang tidak
//! dikenal, dan struktur yang lebih dalam dari [`KEDALAMAN_MAKS`] — pembatas
//! rekursi supaya berkas jahat tidak membuat stack habis.

use std::collections::BTreeMap;

/// Batas kedalaman bersarang. Peta ikon paling dalam ±3; sisanya ditolak
/// lebih dulu, bukan sampai stack habis.
pub const KEDALAMAN_MAKS: usize = 32;

/// Nilai JSON yang sudah diurai.
#[derive(Debug, Clone, PartialEq)]
pub enum Nilai {
    /// `null`
    Null,
    /// `true` atau `false`
    Bool(bool),
    /// Angka. Disimpan sebagai `f64`; [`Nilai::bulat`] yang menolak pecahan.
    Number(f64),
    /// String, dengan escape dan pasangan surrogate sudah diterjemahkan.
    String(String),
    /// Array.
    Array(Vec<Nilai>),
    /// Objek. Bentuk peta karena urutan kunci tidak relevan untuk pembacaan.
    Object(BTreeMap<String, Nilai>),
}

impl Nilai {
    /// Isi objek dengan kunci tertentu, kalau nilai ini memang objek.
    pub fn ambil(&self, kunci: &str) -> Option<&Self> {
        match self {
            Self::Object(peta) => peta.get(kunci),
            _ => None,
        }
    }

    /// Teksnya, kalau memang string.
    pub fn teks(&self) -> Option<&str> {
        match self {
            Self::String(t) => Some(t),
            _ => None,
        }
    }

    /// Angkanya sebagai bilangan bulat, kalau memang angka bulat. Pecahan dan
    /// `NaN` ditolak, bukan dibulatkan diam-diam.
    pub fn bulat(&self) -> Option<i64> {
        match self {
            Self::Number(n) if n.fract() == 0.0 && n.is_finite() => Some(*n as i64),
            _ => None,
        }
    }

    /// Isi array-nya, kalau memang array.
    pub fn array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Benar kalau nilai ini boolean `true`.
    pub fn benar(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}

/// Urai satu dokumen JSON. Pesan galat menyebut posisi dan alasannya.
pub fn urai(teks: &str) -> Result<Nilai, String> {
    let mut p = P {
        b: teks.as_bytes(),
        i: 0,
        kedalaman: 0,
    };
    p.lewati_spasi();
    let nilai = p.nilai()?;
    p.lewati_spasi();
    if p.i != p.b.len() {
        return Err(p.galat("ada karakter tersisa setelah nilai terakhir"));
    }
    Ok(nilai)
}

/// Kursor di atas byte masukan.
struct P<'a> {
    b: &'a [u8],
    i: usize,
    kedalaman: usize,
}

impl<'a> P<'a> {
    fn galat(&self, apa: &str) -> String {
        format!("JSON tidak valid di byte {}: {}", self.i, apa)
    }

    /// Lewati spasi putih. Memindai byte aman: semua byte UTF-8 multi-byte
    /// bernilai ≥ 0x80, jadi tidak akan tertimpa karakter ASCII ini.
    fn lewati_spasi(&mut self) {
        while let Some(c) = self.b.get(self.i) {
            if c.is_ascii_whitespace() {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn nilai(&mut self) -> Result<Nilai, String> {
        match self.byte_berikut()? {
            b'{' => self.objek(),
            b'[' => self.array(),
            b'"' => Ok(Nilai::String(self.teks()?)),
            b't' => self.literal("true", Nilai::Bool(true)),
            b'f' => self.literal("false", Nilai::Bool(false)),
            b'n' => self.literal("null", Nilai::Null),
            b'-' | b'0'..=b'9' => self.angka(),
            lain => Err(self.galat(&format!(
                "nilai tak dikenal dimulai dengan '{}'",
                lain as char
            ))),
        }
    }

    fn byte_berikut(&self) -> Result<u8, String> {
        self.b
            .get(self.i)
            .copied()
            .ok_or_else(|| self.galat("masukan berakhir tiba-tiba"))
    }

    fn masuk(&mut self) -> Result<(), String> {
        self.kedalaman += 1;
        if self.kedalaman > KEDALAMAN_MAKS {
            return Err(self.galat("struktur terlalu dalam"));
        }
        Ok(())
    }

    fn objek(&mut self) -> Result<Nilai, String> {
        self.masuk()?;
        self.i += 1; // '{'
        let mut peta = BTreeMap::new();
        self.lewati_spasi();
        if self.byte_berikut()? == b'}' {
            self.i += 1;
            self.kedalaman -= 1;
            return Ok(Nilai::Object(peta));
        }
        loop {
            self.lewati_spasi();
            if self.byte_berikut()? != b'"' {
                return Err(self.galat("kunci objek harus diapit tanda kutip"));
            }
            let kunci = self.teks()?;
            self.lewati_spasi();
            if self.byte_berikut()? != b':' {
                return Err(self.galat("kunci objek harus diikuti titik dua"));
            }
            self.i += 1;
            self.lewati_spasi();
            let nilai = self.nilai()?;
            peta.insert(kunci, nilai);
            self.lewati_spasi();
            match self.byte_berikut()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    self.kedalaman -= 1;
                    return Ok(Nilai::Object(peta));
                }
                _ => return Err(self.galat("isi objek harus dipisah koma")),
            }
        }
    }

    fn array(&mut self) -> Result<Nilai, String> {
        self.masuk()?;
        self.i += 1; // '['
        let mut isi = Vec::new();
        self.lewati_spasi();
        if self.byte_berikut()? == b']' {
            self.i += 1;
            self.kedalaman -= 1;
            return Ok(Nilai::Array(isi));
        }
        loop {
            self.lewati_spasi();
            isi.push(self.nilai()?);
            self.lewati_spasi();
            match self.byte_berikut()? {
                b',' => self.i += 1,
                b']' => {
                    self.i += 1;
                    self.kedalaman -= 1;
                    return Ok(Nilai::Array(isi));
                }
                _ => return Err(self.galat("isi array harus dipisah koma")),
            }
        }
    }

    fn literal(&mut self, ingin: &str, nilai: Nilai) -> Result<Nilai, String> {
        let akhir = self.i + ingin.len();
        let cocok = self
            .b
            .get(self.i..akhir)
            .is_some_and(|potong| potong == ingin.as_bytes());
        if !cocok {
            return Err(self.galat(&format!("literal '{ingin}' tidak lengkap")));
        }
        self.i = akhir;
        Ok(nilai)
    }

    fn angka(&mut self) -> Result<Nilai, String> {
        let mulai = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        while self.b.get(self.i).is_some_and(|c| {
            c.is_ascii_digit() || *c == b'.' || *c == b'e' || *c == b'E' || *c == b'+' || *c == b'-'
        }) {
            self.i += 1;
        }
        let teks = std::str::from_utf8(&self.b[mulai..self.i])
            .map_err(|_| self.galat("angka bukan UTF-8 yang valid"))?;
        teks.parse::<f64>()
            .map(Nilai::Number)
            .map_err(|_| self.galat("angka tidak bisa dibaca"))
    }

    fn teks(&mut self) -> Result<String, String> {
        self.i += 1; // '"' pembuka
        let mut keluar = String::new();
        loop {
            let c = self.byte_berikut()?;
            self.i += 1;
            match c {
                b'"' => return Ok(keluar),
                b'\\' => {
                    let kode = self.byte_berikut()?;
                    self.i += 1;
                    let karakter = match kode {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{0008}',
                        b'f' => '\u{000c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.escape_unicode()?,
                        _ => return Err(self.galat("escape tidak dikenal")),
                    };
                    keluar.push(karakter);
                }
                _ => {
                    // Salin byte mentahnya; UTF-8 divalidasi sekaligus di sini.
                    let mulai = self.i - 1;
                    let panjang = self.panjang_utf8(mulai);
                    if self.i + panjang > self.b.len() {
                        return Err(self.galat("string berakhir di tengah karakter UTF-8"));
                    }
                    keluar.push_str(
                        std::str::from_utf8(&self.b[mulai..mulai + panjang])
                            .map_err(|_| self.galat("string bukan UTF-8 yang valid"))?,
                    );
                    self.i = mulai + panjang;
                }
            }
        }
    }

    /// `\uXXXX`, termasuk pasangan surrogate untuk karakter di luar BMP.
    fn escape_unicode(&mut self) -> Result<char, String> {
        let unit = self.hex4()?;
        // Surrogate tinggi harus disusul `\u` surrogate rendah.
        if (0xD800..0xDC00).contains(&unit) {
            if self.b.get(self.i) != Some(&b'\\') || self.b.get(self.i + 1) != Some(&b'u') {
                return Err(self.galat("surrogate tinggi tanpa pasangan"));
            }
            self.i += 2;
            let bawah = self.hex4()?;
            if !(0xDC00..0xE000).contains(&bawah) {
                return Err(self.galat("pasangan surrogate tidak valid"));
            }
            let titik = 0x10000 + ((unit - 0xD800) << 10) + (bawah - 0xDC00);
            return char::from_u32(titik).ok_or_else(|| self.galat("nilai surrogate tidak valid"));
        }
        char::from_u32(unit).ok_or_else(|| self.galat("nilai \\u bukan karakter yang valid"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let potong = self
            .b
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.galat("escape \\u terpotong"))?;
        let teks = std::str::from_utf8(potong).map_err(|_| self.galat("escape \\u bukan ASCII"))?;
        let nilai = u32::from_str_radix(teks, 16)
            .map_err(|_| self.galat("escape \\u bukan heksadesimal"))?;
        self.i += 4;
        Ok(nilai)
    }

    /// Panjang karakter UTF-8 yang mulai di `mulai`, dari byte pertamanya.
    fn panjang_utf8(&self, mulai: usize) -> usize {
        match self.b.get(mulai) {
            Some(0x00..=0x7F) => 1,
            Some(0xC0..=0xDF) => 2,
            Some(0xE0..=0xEF) => 3,
            Some(0xF0..=0xF7) => 4,
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{urai, KEDALAMAN_MAKS};

    #[test]
    fn objek_sederhana_diurai() {
        let nilai =
            urai(r#"{"a": 1, "b": "dua", "c": true, "d": null}"#).expect("harus bisa diurai");
        assert_eq!(nilai.ambil("a").and_then(super::Nilai::bulat), Some(1));
        assert_eq!(nilai.ambil("b").and_then(super::Nilai::teks), Some("dua"));
        assert!(nilai.ambil("c").is_some_and(super::Nilai::benar));
        assert_eq!(nilai.ambil("d"), Some(&super::Nilai::Null));
    }

    #[test]
    fn escape_dan_unicode_diterjemahkan() {
        let nilai = urai(r#"{"s": "a\"b\\c\ndA🎉"}"#).expect("harus bisa diurai");
        assert_eq!(
            nilai.ambil("s").and_then(super::Nilai::teks),
            Some("a\"b\\c\ndA\u{1f389}")
        );
    }

    #[test]
    fn array_dan_kosong_benar() {
        let nilai = urai("[1, 2, [3, {}], []]").expect("harus bisa diurai");
        let isi = nilai.array().expect("harus array");
        assert_eq!(isi.len(), 4);
        assert_eq!(nilai.array().map(<[_]>::len), Some(4));
    }

    #[test]
    fn masukan_rusak_ditolak() {
        for rusak in [
            "{",
            "{\"a\" 1}",
            "{\"a\": 1,}",
            "[1, 2",
            "tru",
            "{\"a\": 1} sisa",
            "\"belumutup",
            "{\"a\": 1e}",
            "{\"a\": \"\\q\"}",
        ] {
            assert!(urai(rusak).is_err(), "harusnya ditolak: {rusak}");
        }
    }

    #[test]
    fn kedalaman_dibatasi() {
        let dalam = format!(
            "{}{}",
            "[".repeat(KEDALAMAN_MAKS + 2),
            "]".repeat(KEDALAMAN_MAKS + 2)
        );
        let pesan = urai(&dalam).expect_err("harus ditolak");
        assert!(pesan.contains("terlalu dalam"), "{pesan}");
    }

    #[test]
    fn pecahan_bukan_bulat_ditolak_saat_diminta() {
        let nilai = urai("1.5").expect("harus bisa diurai");
        assert_eq!(nilai.bulat(), None);
        assert_eq!(urai("2").expect("harus bisa diurai").bulat(), Some(2));
    }
}
