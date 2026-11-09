//! A PDF written by hand: A4 pages with a title, paragraphs, a table, a picture and page numbers.
//! It only uses the fonts every reader has, so it needs no library.

use std::fmt::Write as _;

use crate::{document::Words, text::Person};

const WIDTH: f32 = 595.0;
const HEIGHT: f32 = 842.0;
const MARGIN: f32 = 56.0;
/// Characters of Helvetica 11 pt that fit across the page, on average.
const LINE_CHARS: usize = 88;

/// Text in the encoding of the standard fonts (`WinAnsiEncoding`), escaped for a PDF string.
fn pdf_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let byte: u8 = match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                u8::try_from(c).unwrap_or(b'?')
            }
            '€' => 0x80,
            '…' => 0x85,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '–' => 0x96,
            '—' => 0x97,
            'Œ' => 0x8C,
            'œ' => 0x9C,
            c if u32::from(c) < 0x80 || (0xA0..=0xFF).contains(&u32::from(c)) => {
                u8::try_from(u32::from(c)).unwrap_or(b'?')
            }
            _ => b'?',
        };
        if byte < 0x80 {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "\\{byte:03o}");
        }
    }
    out
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        let line = lines.last_mut().map_or(0, |line| line.chars().count());
        if line > 0 && line + 1 + word.chars().count() > width {
            lines.push(String::new());
        }
        if let Some(line) = lines.last_mut() {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
    }
    lines
}

struct Page {
    stream: String,
    y: f32,
}

impl Page {
    fn new() -> Self {
        Self {
            stream: String::new(),
            y: HEIGHT - MARGIN,
        }
    }

    fn text(&mut self, font: &str, size: f32, x: f32, y: f32, text: &str) {
        let _ = writeln!(
            self.stream,
            "BT /{font} {size} Tf {x:.1} {y:.1} Td ({}) Tj ET",
            pdf_text(text)
        );
    }

    fn heading(&mut self, size: f32, text: &str) {
        self.y -= size;
        self.text("F2", size, MARGIN, self.y, text);
        self.y -= size * 0.6;
    }

    fn paragraph(&mut self, text: &str) {
        for line in wrap(text, LINE_CHARS) {
            self.y -= 15.0;
            self.text("F1", 11.0, MARGIN, self.y, &line);
        }
        self.y -= 10.0;
    }

    fn table(&mut self, header: &[&str; 3], rows: &[[String; 3]]) {
        let widths = [150.0, 210.0, 123.0];
        let row_height = 20.0;
        let all = std::iter::once(header.map(str::to_string)).chain(rows.iter().cloned());
        for (index, row) in all.enumerate() {
            self.y -= row_height;
            let mut x = MARGIN;
            for (column, cell) in row.iter().enumerate() {
                let _ = writeln!(
                    self.stream,
                    "0.6 G {x:.1} {:.1} {:.1} {row_height:.1} re S",
                    self.y, widths[column]
                );
                self.text(
                    if index == 0 { "F2" } else { "F1" },
                    10.0,
                    x + 6.0,
                    self.y + 6.5,
                    cell,
                );
                x += widths[column];
            }
        }
        self.y -= 18.0;
    }

    fn picture(&mut self, width: f32, height: f32) {
        self.y -= height;
        let _ = writeln!(
            self.stream,
            "q {width:.1} 0 0 {height:.1} {MARGIN:.1} {:.1} cm /Im1 Do Q",
            self.y
        );
        self.y -= 18.0;
    }

    fn number(&mut self, number: usize, total: usize) {
        self.text(
            "F1",
            9.0,
            WIDTH / 2.0 - 12.0,
            MARGIN / 2.0,
            &format!("{number} / {total}"),
        );
    }
}

/// A PDF of `pages` pages; `jpeg` is a picture of `jpeg_size` pixels.
pub fn write(pages: usize, words: &Words, jpeg: &[u8], jpeg_size: (u32, u32)) -> Vec<u8> {
    let pages = pages.max(1);
    let mut contents = Vec::new();
    for number in 1..=pages {
        let mut page = Page::new();
        if number == 1 {
            page.heading(24.0, words.title);
            page.paragraph(words.paragraphs[0]);
            page.paragraph(words.paragraphs[1]);
            page.heading(14.0, words.table_title);
            let rows: Vec<[String; 3]> = (0..5)
                .map(|index| {
                    let person = Person::new(index, words.language);
                    [
                        format!("{} {}", person.first, person.last),
                        person.email.clone(),
                        person.city.to_string(),
                    ]
                })
                .collect();
            page.table(&words.table_header, &rows);
            let width = 300.0;
            #[allow(clippy::cast_precision_loss)]
            let height = width * jpeg_size.1 as f32 / jpeg_size.0 as f32;
            page.picture(width, height);
        } else {
            page.heading(16.0, &format!("{} {number}", words.section));
            for paragraph in words.paragraphs.iter().cycle().skip(number).take(5) {
                page.paragraph(paragraph);
            }
        }
        page.number(number, pages);
        contents.push(page.stream);
    }

    let mut out: Vec<u8> = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    let mut object = |out: &mut Vec<u8>, body: &[u8]| {
        offsets.push(out.len());
        let number = offsets.len();
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    // 1 catalog, 2 page tree, 3 and 4 fonts, 5 picture, then a content and a page per page.
    let kids: Vec<String> = (0..pages)
        .map(|index| format!("{} 0 R", 7 + index * 2))
        .collect();
    object(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>");
    object(
        &mut out,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            kids.join(" ")
        )
        .as_bytes(),
    );
    object(
        &mut out,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    object(
        &mut out,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    );
    let mut picture = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        jpeg_size.0, jpeg_size.1, jpeg.len()
    )
    .into_bytes();
    picture.extend_from_slice(jpeg);
    picture.extend_from_slice(b"\nendstream");
    object(&mut out, &picture);
    for (index, content) in contents.iter().enumerate() {
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content.as_bytes());
        stream.extend_from_slice(b"endstream");
        object(&mut out, &stream);
        object(
            &mut out,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {WIDTH} {HEIGHT}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> /XObject << /Im1 5 0 R >> >> /Contents {} 0 R >>",
                6 + index * 2
            )
            .as_bytes(),
        );
    }
    let xref = out.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in &offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len() + 1
    );
    out.extend_from_slice(table.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Language;

    #[test]
    fn writes_a_readable_structure() {
        let pdf = write(3, &Words::new(Language::French), b"\xFF\xD8fake", (4, 3));
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Count 3"));
        assert!(text.ends_with("%%EOF\n"));
        // Each offset of the table points at its object.
        let xref = text.rfind("xref\n").unwrap();
        for (number, line) in text[xref..].lines().skip(3).take(11).enumerate() {
            let offset: usize = line[..10].parse().unwrap();
            assert!(pdf[offset..].starts_with(format!("{} 0 obj", number + 1).as_bytes()));
        }
    }

    #[test]
    fn encodes_french_text_for_the_standard_fonts() {
        assert_eq!(pdf_text("é (1)"), "\\351 \\(1\\)");
        assert_eq!(pdf_text("l’été…"), "l\\222\\351t\\351\\205");
    }

    #[test]
    fn wraps_long_paragraphs() {
        let lines = wrap(&"mot ".repeat(60), 20);
        assert!(lines.iter().all(|line| line.chars().count() <= 20));
        assert_eq!(lines.join(" "), "mot ".repeat(60).trim());
    }
}
