//! Office documents written by hand: the smallest set of parts that office suites open without
//! complaint, with the same words as the PDF.

use std::{
    fmt::Write as _,
    io::{Cursor, Write},
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::text::{Language, Person};

/// The words of every document, in the language of the interface.
pub struct Words {
    pub language: Language,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub paragraphs: [&'static str; 3],
    pub list: [&'static str; 3],
    pub table_title: &'static str,
    pub picture_title: &'static str,
    pub table_header: [&'static str; 3],
    pub section: &'static str,
    pub sheet: &'static str,
    pub columns: [&'static str; 7],
    pub total: &'static str,
}

impl Words {
    pub fn new(language: Language) -> Self {
        match language {
            Language::French => Self {
                language,
                title: "Document d’exemple",
                subtitle: "Créé par Template Example",
                paragraphs: [
                    "Ce document sert à tester un logiciel, un site ou un envoi de fichiers. Son contenu est fictif : les personnes, les adresses et les montants n’existent pas.",
                    "Le vif renard brun saute par-dessus le chien paresseux. Portez ce vieux whisky au juge blond qui fume. Les accents é, è, à, ù et ç, le symbole € et les guillemets « » sont là pour vérifier l’encodage.",
                    "Un troisième paragraphe, plus court, pour remplir les pages suivantes et tester la pagination.",
                ],
                list: ["Premier élément", "Deuxième élément", "Troisième élément"],
                table_title: "Un tableau",
                picture_title: "Une image",
                table_header: ["Nom", "E-mail", "Ville"],
                section: "Section",
                sheet: "Personnes",
                columns: [
                    "id",
                    "Prénom",
                    "Nom",
                    "E-mail",
                    "Ville",
                    "Inscription",
                    "Montant",
                ],
                total: "Total",
            },
            Language::English => Self {
                language,
                title: "Sample document",
                subtitle: "Created by Template Example",
                paragraphs: [
                    "This document is for testing an app, a website or a file upload. Its content is fictitious: the people, addresses and amounts do not exist.",
                    "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. The accents é, è, à, ù and ç, the € sign and the quotes « » are here to check the encoding.",
                    "A third, shorter paragraph, to fill the next pages and test the pagination.",
                ],
                list: ["First item", "Second item", "Third item"],
                table_title: "A table",
                picture_title: "A picture",
                table_header: ["Name", "Email", "City"],
                section: "Section",
                sheet: "People",
                columns: [
                    "id",
                    "First name",
                    "Last name",
                    "Email",
                    "City",
                    "Signed up",
                    "Amount",
                ],
                total: "Total",
            },
        }
    }
}

fn xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn zip(parts: &[(&str, &[u8])], stored_first: bool) -> Result<Vec<u8>, String> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (index, (name, bytes)) in parts.iter().enumerate() {
        // An OpenDocument starts with its `mimetype`, uncompressed.
        let method = if stored_first && index == 0 {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(method),
            )
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    Ok(writer
        .finish()
        .map_err(|error| error.to_string())?
        .into_inner())
}

const RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const OFFICE_RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn package_rels(target: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/officeDocument" Target="{target}"/></Relationships>"#
    )
}

/// English Metric Units: what Office measures pictures in.
const EMU_PER_PIXEL: u64 = 9525;

pub fn docx(
    pages: usize,
    words: &Words,
    jpeg: &[u8],
    jpeg_size: (u32, u32),
) -> Result<Vec<u8>, String> {
    let paragraph = |text: &str| {
        format!(
            "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml(text)
        )
    };
    let styled = |text: &str, size: u32| {
        format!(
            "<w:p><w:pPr><w:spacing w:after=\"160\"/></w:pPr><w:r><w:rPr><w:b/><w:sz w:val=\"{size}\"/></w:rPr><w:t>{}</w:t></w:r></w:p>",
            xml(text)
        )
    };
    let mut body = styled(words.title, 48);
    body += &styled(words.subtitle, 28);
    body += &paragraph(words.paragraphs[0]);
    body += &paragraph(words.paragraphs[1]);
    for item in words.list {
        let _ = write!(
            body,
            "<w:p><w:pPr><w:ind w:left=\"360\"/></w:pPr><w:r><w:t xml:space=\"preserve\">•  {}</w:t></w:r></w:p>",
            xml(item)
        );
    }
    body += &styled(words.table_title, 28);
    let cell = |text: &str, bold: bool| {
        format!(
            "<w:tc><w:tcPr><w:tcW w:w=\"3000\" w:type=\"dxa\"/></w:tcPr><w:p><w:r>{}<w:t>{}</w:t></w:r></w:p></w:tc>",
            if bold { "<w:rPr><w:b/></w:rPr>" } else { "" },
            xml(text)
        )
    };
    body += "<w:tbl><w:tblPr><w:tblW w:w=\"9000\" w:type=\"dxa\"/><w:tblBorders>";
    for side in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        let _ = write!(
            body,
            "<w:{side} w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"999999\"/>"
        );
    }
    body += "</w:tblBorders></w:tblPr><w:tblGrid><w:gridCol w:w=\"3000\"/><w:gridCol w:w=\"3000\"/><w:gridCol w:w=\"3000\"/></w:tblGrid>";
    let _ = write!(
        body,
        "<w:tr>{}</w:tr>",
        words
            .table_header
            .iter()
            .map(|text| cell(text, true))
            .collect::<Vec<_>>()
            .concat()
    );
    for index in 0..5 {
        let p = Person::new(index, words.language);
        let _ = write!(
            body,
            "<w:tr>{}{}{}</w:tr>",
            cell(&format!("{} {}", p.first, p.last), false),
            cell(&p.email, false),
            cell(p.city, false)
        );
    }
    body += "</w:tbl>";
    let (cx, cy) = (
        u64::from(jpeg_size.0) * EMU_PER_PIXEL / 2,
        u64::from(jpeg_size.1) * EMU_PER_PIXEL / 2,
    );
    let _ = write!(
        body,
        r#"<w:p><w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="1" name="Picture 1"/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="picture.jpeg"/><pic:cNvPicPr/></pic:nvPicPr><pic:blipFill><a:blip r:embed="rId1"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"#
    );
    for number in 2..=pages {
        body += "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>";
        body += &styled(&format!("{} {number}", words.section), 32);
        for text in words.paragraphs.iter().cycle().skip(number).take(5) {
            body += &paragraph(text);
        }
    }
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="{OFFICE_RELS}" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>{body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134" w:header="0" w:footer="0" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    );
    let types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="jpeg" ContentType="image/jpeg"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;
    let document_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/image" Target="media/picture.jpeg"/></Relationships>"#
    );
    zip(
        &[
            ("[Content_Types].xml", types.as_bytes()),
            ("_rels/.rels", package_rels("word/document.xml").as_bytes()),
            ("word/document.xml", document.as_bytes()),
            ("word/_rels/document.xml.rels", document_rels.as_bytes()),
            ("word/media/picture.jpeg", jpeg),
        ],
        false,
    )
}

/// Days from 1899-12-30, the start of Excel's calendar, to 1 January 2026.
const EXCEL_2026: u32 = 46_023;

fn column(index: usize) -> char {
    char::from(b'A' + u8::try_from(index).unwrap_or(0))
}

pub fn xlsx(rows: usize, words: &Words) -> Result<Vec<u8>, String> {
    let rows = rows.max(1);
    let text = |reference: String, value: &str, style: u8| {
        format!(
            "<c r=\"{reference}\" t=\"inlineStr\" s=\"{style}\"><is><t>{}</t></is></c>",
            xml(value)
        )
    };
    let mut data = String::from("<row r=\"1\">");
    for (index, title) in words.columns.iter().enumerate() {
        data += &text(format!("{}1", column(index)), title, 1);
    }
    data += "</row>";
    for index in 0..rows {
        let p = Person::new(index, words.language);
        let row = index + 2;
        let _ = write!(
            data,
            "<row r=\"{row}\"><c r=\"A{row}\"><v>{}</v></c>{}{}{}{}<c r=\"F{row}\" s=\"2\"><v>{}</v></c><c r=\"G{row}\" s=\"3\"><v>{}</v></c></row>",
            p.id,
            text(format!("B{row}"), p.first, 0),
            text(format!("C{row}"), p.last, 0),
            text(format!("D{row}"), &p.email, 0),
            text(format!("E{row}"), p.city, 0),
            EXCEL_2026 + p.day,
            p.amount(),
        );
    }
    let total = rows + 2;
    let _ = write!(
        data,
        "<row r=\"{total}\">{}<c r=\"D{total}\" s=\"1\"><f>COUNTA(D2:D{})</f></c><c r=\"G{total}\" s=\"4\"><f>SUM(G2:G{})</f></c></row>",
        text(format!("A{total}"), words.total, 1),
        rows + 1,
        rows + 1
    );
    let currency = match words.language {
        Language::French => "#,##0.00\\ &quot;€&quot;",
        Language::English => "&quot;$&quot;#,##0.00",
    };
    let sheet = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><cols><col min="1" max="1" width="6" customWidth="1"/><col min="2" max="3" width="14" customWidth="1"/><col min="4" max="4" width="34" customWidth="1"/><col min="5" max="7" width="14" customWidth="1"/></cols><sheetData>{data}</sheetData></worksheet>"#
    );
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="2"><numFmt numFmtId="164" formatCode="yyyy-mm-dd"/><numFmt numFmtId="165" formatCode="{currency}"/></numFmts><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="5"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="1"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/><xf numFmtId="165" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/><xf numFmtId="165" fontId="1" fillId="0" borderId="0" xfId="0" applyNumberFormat="1" applyFont="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"#
    );
    let workbook = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{OFFICE_RELS}"><sheets><sheet name="{}" sheetId="1" r:id="rId1"/></sheets><calcPr calcId="191029" fullCalcOnLoad="1"/></workbook>"#,
        xml(words.sheet)
    );
    let workbook_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{OFFICE_RELS}/styles" Target="styles.xml"/></Relationships>"#
    );
    let types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"#;
    zip(
        &[
            ("[Content_Types].xml", types.as_bytes()),
            ("_rels/.rels", package_rels("xl/workbook.xml").as_bytes()),
            ("xl/workbook.xml", workbook.as_bytes()),
            ("xl/_rels/workbook.xml.rels", workbook_rels.as_bytes()),
            ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
            ("xl/styles.xml", styles.as_bytes()),
        ],
        false,
    )
}

const PPT_NAMESPACES: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const EMPTY_TREE: &str = r#"<p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>"#;

fn theme() -> String {
    let colors = [
        ("dk1", "000000"),
        ("lt1", "FFFFFF"),
        ("dk2", "1F2328"),
        ("lt2", "E7E6E6"),
        ("accent1", "2F6FEB"),
        ("accent2", "E36C09"),
        ("accent3", "7F7F7F"),
        ("accent4", "C9A000"),
        ("accent5", "4BACC6"),
        ("accent6", "70AD47"),
        ("hlink", "0563C1"),
        ("folHlink", "954F72"),
    ];
    let scheme = colors
        .iter()
        .map(|(name, hex)| format!("<a:{name}><a:srgbClr val=\"{hex}\"/></a:{name}>"))
        .collect::<Vec<_>>()
        .concat();
    let fill = "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>";
    let line = "<a:ln w=\"9525\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill></a:ln>";
    let effect = "<a:effectStyle><a:effectLst/></a:effectStyle>";
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Arthex"><a:themeElements><a:clrScheme name="Arthex">{scheme}</a:clrScheme><a:fontScheme name="Arthex"><a:majorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Arthex"><a:fillStyleLst>{fill}{fill}{fill}</a:fillStyleLst><a:lnStyleLst>{line}{line}{line}</a:lnStyleLst><a:effectStyleLst>{effect}{effect}{effect}</a:effectStyleLst><a:bgFillStyleLst>{fill}{fill}{fill}</a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#
    )
}

fn text_box(id: u32, (x, y, cx, cy): (u64, u64, u64, u64), paragraphs: &str) -> String {
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Text {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square"/><a:lstStyle/>{paragraphs}</p:txBody></p:sp>"#
    )
}

fn run(text: &str, size: u32, bold: bool) -> String {
    format!(
        "<a:p><a:r><a:rPr lang=\"en-US\" sz=\"{size}\" b=\"{}\"/><a:t>{}</a:t></a:r></a:p>",
        u8::from(bold),
        xml(text)
    )
}

fn bullet(text: &str) -> String {
    format!(
        "<a:p><a:pPr marL=\"342900\" indent=\"-342900\"><a:buFont typeface=\"Arial\"/><a:buChar char=\"•\"/></a:pPr><a:r><a:rPr lang=\"en-US\" sz=\"2400\"/><a:t>{}</a:t></a:r></a:p>",
        xml(text)
    )
}

#[allow(clippy::too_many_lines)] // One part after the other, as the package lists them.
pub fn pptx(
    slides: usize,
    words: &Words,
    jpeg: &[u8],
    jpeg_size: (u32, u32),
) -> Result<Vec<u8>, String> {
    let slides = slides.max(1);
    let title_area = (838_200, 365_125, 10_515_600, 1_325_563);
    let body_area = (838_200, 1_825_625, 10_515_600, 4_351_338);
    let mut slide_parts = Vec::new();
    for number in 1..=slides {
        let mut has_picture = false;
        let shapes = match number {
            1 => {
                text_box(
                    2,
                    (838_200, 2_130_000, 10_515_600, 1_470_000),
                    &run(words.title, 5400, true),
                ) + &text_box(
                    3,
                    (838_200, 3_700_000, 10_515_600, 900_000),
                    &run(words.subtitle, 2800, false),
                )
            }
            3 => {
                has_picture = true;
                let height = 4_000_000 * u64::from(jpeg_size.1) / u64::from(jpeg_size.0.max(1));
                text_box(2, title_area, &run(words.picture_title, 4000, true))
                    + &format!(
                        r#"<p:pic><p:nvPicPr><p:cNvPr id="3" name="Picture"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="4096000" y="1825625"/><a:ext cx="4000000" cy="{height}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#
                    )
            }
            _ => {
                text_box(
                    2,
                    title_area,
                    &run(&format!("{} {number}", words.section), 4000, true),
                ) + &text_box(
                    3,
                    body_area,
                    &words
                        .list
                        .iter()
                        .map(|item| bullet(item))
                        .collect::<Vec<_>>()
                        .concat(),
                )
            }
        };
        let slide = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sld {PPT_NAMESPACES}><p:cSld>{EMPTY_TREE}{shapes}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>"#
        );
        let picture = if has_picture {
            format!(
                r#"<Relationship Id="rId2" Type="{OFFICE_RELS}/image" Target="../media/picture.jpeg"/>"#
            )
        } else {
            String::new()
        };
        let rels = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>{picture}</Relationships>"#
        );
        slide_parts.push((
            format!("ppt/slides/slide{number}.xml"),
            slide,
            format!("ppt/slides/_rels/slide{number}.xml.rels"),
            rels,
        ));
    }
    let slide_ids = (1..=slides)
        .map(|number| {
            format!(
                "<p:sldId id=\"{}\" r:id=\"rId{}\"/>",
                255 + number,
                number + 2
            )
        })
        .collect::<Vec<_>>()
        .concat();
    let presentation = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation {PPT_NAMESPACES} saveSubsetFonts="1"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst>{slide_ids}</p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>"#
    );
    let slide_rels = (1..=slides).map(|number| format!(r#"<Relationship Id="rId{}" Type="{OFFICE_RELS}/slide" Target="slides/slide{number}.xml"/>"#, number + 2)).collect::<Vec<_>>().concat();
    let presentation_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="{OFFICE_RELS}/theme" Target="theme/theme1.xml"/>{slide_rels}</Relationships>"#
    );
    let master = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldMaster {PPT_NAMESPACES}><p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef></p:bg>{EMPTY_TREE}</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>"#
    );
    let master_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/slideLayout" Target="../slideLayouts/slideLayout1.xml"/><Relationship Id="rId2" Type="{OFFICE_RELS}/theme" Target="../theme/theme1.xml"/></Relationships>"#
    );
    let layout = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:sldLayout {PPT_NAMESPACES} type="blank" preserve="1"><p:cSld name="Blank">{EMPTY_TREE}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#
    );
    let layout_rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="{RELS}"><Relationship Id="rId1" Type="{OFFICE_RELS}/slideMaster" Target="../slideMasters/slideMaster1.xml"/></Relationships>"#
    );
    let slide_types = (1..=slides).map(|number| format!(r#"<Override PartName="/ppt/slides/slide{number}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#)).collect::<Vec<_>>().concat();
    let types = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="jpeg" ContentType="image/jpeg"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/><Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>{slide_types}</Types>"#
    );
    let theme = theme();
    let package_rels = package_rels("ppt/presentation.xml");
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", package_rels.as_bytes()),
        ("ppt/presentation.xml", presentation.as_bytes()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_rels.as_bytes(),
        ),
        ("ppt/slideMasters/slideMaster1.xml", master.as_bytes()),
        (
            "ppt/slideMasters/_rels/slideMaster1.xml.rels",
            master_rels.as_bytes(),
        ),
        ("ppt/slideLayouts/slideLayout1.xml", layout.as_bytes()),
        (
            "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
            layout_rels.as_bytes(),
        ),
        ("ppt/theme/theme1.xml", theme.as_bytes()),
        ("ppt/media/picture.jpeg", jpeg),
    ];
    for (slide_name, slide, rels_name, rels) in &slide_parts {
        parts.push((slide_name, slide.as_bytes()));
        parts.push((rels_name, rels.as_bytes()));
    }
    zip(&parts, false)
}

pub fn odt(pages: usize, words: &Words) -> Result<Vec<u8>, String> {
    let paragraph = |text: &str| format!("<text:p>{}</text:p>", xml(text));
    let mut body = format!(
        "<text:h text:style-name=\"Title\" text:outline-level=\"1\">{}</text:h>",
        xml(words.title)
    );
    body += &paragraph(words.paragraphs[0]);
    body += &paragraph(words.paragraphs[1]);
    body += "<text:list>";
    for item in words.list {
        let _ = write!(body, "<text:list-item>{}</text:list-item>", paragraph(item));
    }
    body += "</text:list>";
    let _ = write!(
        body,
        "<text:h text:style-name=\"Heading\" text:outline-level=\"2\">{}</text:h>",
        xml(words.table_title)
    );
    body += "<table:table table:name=\"Table1\"><table:table-column table:number-columns-repeated=\"3\"/>";
    let row = |cells: [String; 3]| {
        format!(
            "<table:table-row>{}</table:table-row>",
            cells
                .iter()
                .map(|cell| format!(
                    "<table:table-cell office:value-type=\"string\">{}</table:table-cell>",
                    paragraph(cell)
                ))
                .collect::<Vec<_>>()
                .concat()
        )
    };
    body += &row(words.table_header.map(str::to_string));
    for index in 0..5 {
        let p = Person::new(index, words.language);
        body += &row([
            format!("{} {}", p.first, p.last),
            p.email.clone(),
            p.city.to_string(),
        ]);
    }
    body += "</table:table>";
    for number in 2..=pages {
        let _ = write!(
            body,
            "<text:h text:style-name=\"Break\" text:outline-level=\"2\">{} {number}</text:h>",
            words.section
        );
        for text in words.paragraphs.iter().cycle().skip(number).take(5) {
            body += &paragraph(text);
        }
    }
    let content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3"><office:automatic-styles><style:style style:name="Title" style:family="paragraph"><style:text-properties fo:font-size="22pt" fo:font-weight="bold"/></style:style><style:style style:name="Heading" style:family="paragraph"><style:text-properties fo:font-size="15pt" fo:font-weight="bold"/></style:style><style:style style:name="Break" style:family="paragraph"><style:paragraph-properties fo:break-before="page"/><style:text-properties fo:font-size="15pt" fo:font-weight="bold"/></style:style></office:automatic-styles><office:body><office:text>{body}</office:text></office:body></office:document-content>"#
    );
    let manifest = r#"<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/></manifest:manifest>"#;
    zip(
        &[
            ("mimetype", b"application/vnd.oasis.opendocument.text"),
            ("META-INF/manifest.xml", manifest.as_bytes()),
            ("content.xml", content.as_bytes()),
        ],
        true,
    )
}

/// Rows of a spreadsheet for a number of printed pages: about forty per page.
pub fn rows_for_pages(pages: usize) -> usize {
    pages.max(1) * 40
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    fn part(bytes: &[u8], name: &str) -> String {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        text
    }

    #[test]
    fn builds_each_office_document() {
        let words = Words::new(Language::French);
        let docx = docx(3, &words, b"jpeg", (900, 600)).unwrap();
        assert!(part(&docx, "word/document.xml").contains("Document d’exemple"));
        assert_eq!(
            part(&docx, "word/document.xml")
                .matches("w:type=\"page\"")
                .count(),
            2
        );
        let xlsx = xlsx(40, &words).unwrap();
        assert!(part(&xlsx, "xl/worksheets/sheet1.xml").contains("<f>SUM(G2:G41)</f>"));
        let pptx = pptx(5, &words, b"jpeg", (900, 600)).unwrap();
        assert!(part(&pptx, "ppt/presentation.xml").contains("rId7"));
        assert!(part(&pptx, "ppt/slides/_rels/slide3.xml.rels").contains("picture.jpeg"));
        let odt = odt(2, &words).unwrap();
        assert_eq!(
            part(&odt, "mimetype"),
            "application/vnd.oasis.opendocument.text"
        );
    }

    #[test]
    fn escapes_text_for_xml() {
        assert_eq!(xml("a < b & \"c\""), "a &lt; b &amp; &quot;c&quot;");
    }

    #[test]
    fn fills_forty_rows_a_page() {
        assert_eq!(rows_for_pages(5), 200);
        assert_eq!(rows_for_pages(0), 40);
    }
}
