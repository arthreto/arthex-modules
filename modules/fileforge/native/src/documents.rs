//! Documents go through Word or `LibreOffice` when one is installed: a faithful PDF to Word conversion
//! takes a whole office suite, far more than a module could carry. Compression needs neither.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use lopdf::Document;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

/// The path and format come through the environment, so no file name can become a command.
const WORD_SCRIPT: &str = "$ErrorActionPreference='Stop';\
$w=New-Object -ComObject Word.Application;\
$w.Visible=$false;$w.DisplayAlerts=0;\
try{$d=$w.Documents.Open($env:FILEFORGE_INPUT,$false,$true);\
$d.SaveAs2($env:FILEFORGE_OUTPUT,[int]$env:FILEFORGE_FORMAT);\
$d.Close(0)}finally{$w.Quit()}";

#[derive(Debug, PartialEq, Eq)]
pub enum Office {
    Word,
    LibreOffice(PathBuf),
}

fn word_format(target: &str) -> Option<&'static str> {
    match target {
        "pdf" => Some("17"),
        "docx" => Some("16"),
        "odt" => Some("23"),
        "txt" => Some("2"),
        "html" => Some("10"),
        _ => None,
    }
}

fn word_opens(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "pdf" | "docx" | "doc" | "odt" | "rtf" | "txt" | "html" | "htm"
    )
}

#[cfg(windows)]
fn has_word() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT)
        .open_subkey("Word.Application")
        .is_ok()
}

#[cfg(not(windows))]
fn has_word() -> bool {
    false
}

fn libre_office() -> Option<PathBuf> {
    ["ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|dir| Path::new(&dir).join(r"LibreOffice\program\soffice.exe"))
        .find(|path| path.is_file())
}

/// What can convert a file of this extension on this PC.
pub fn find_office(extension: &str) -> Option<Office> {
    if word_opens(extension) && has_word() {
        return Some(Office::Word);
    }
    libre_office().map(Office::LibreOffice)
}

fn libre_filter(target: &str) -> &str {
    if target == "txt" { "txt:Text" } else { target }
}

/// A command that writes `output` (Word) or `<input name>.<target>` in `output`'s folder.
pub fn command(office: &Office, input: &Path, output: &Path, target: &str) -> Command {
    match office {
        Office::Word => {
            let mut command = Command::new("powershell");
            command
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                ])
                .arg(WORD_SCRIPT)
                .env("FILEFORGE_INPUT", input)
                .env("FILEFORGE_OUTPUT", output)
                .env("FILEFORGE_FORMAT", word_format(target).unwrap_or("16"));
            command
        }
        Office::LibreOffice(program) => {
            let mut command = Command::new(program);
            command.args(["--headless", "--norestore"]);
            let is_pdf = input
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"));
            if is_pdf {
                command.arg("--infilter=writer_pdf_import");
            }
            command
                .arg("--convert-to")
                .arg(libre_filter(target))
                .arg("--outdir")
                .arg(output.parent().unwrap_or(Path::new(".")))
                .arg(input);
            command
        }
    }
}

/// Rewrites a PDF with its streams compressed and its unused objects gone. Nothing is lost.
pub fn compress_pdf(input: &Path, output: &Path) -> Result<(), String> {
    let mut document = Document::load(input).map_err(|error| error.to_string())?;
    document.prune_objects();
    document.compress();
    document.save(output).map_err(|error| error.to_string())?;
    Ok(())
}

fn is_jpeg_inside(name: &str) -> bool {
    let is_jpeg = Path::new(name).extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg")
    });
    is_jpeg && name.contains("media/")
}

/// Office files are zip archives: rewrites one tighter, with `shrink` applied to its photos.
pub fn compress_office(
    input: &Path,
    output: &Path,
    shrink: &mut dyn FnMut(&[u8]) -> Option<Vec<u8>>,
) -> Result<(), String> {
    let text = |error: &dyn std::fmt::Display| error.to_string();
    let source = File::open(input).map_err(|error| text(&error))?;
    let mut archive = ZipArchive::new(source).map_err(|error| text(&error))?;
    let target = File::create(output).map_err(|error| text(&error))?;
    let mut writer = ZipWriter::new(target);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| text(&error))?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| text(&error))?;
        if is_jpeg_inside(&name)
            && let Some(smaller) = shrink(&bytes)
            && smaller.len() < bytes.len()
        {
            bytes = smaller;
        }
        writer
            .start_file(name, options)
            .map_err(|error| text(&error))?;
        writer.write_all(&bytes).map_err(|error| text(&error))?;
    }
    writer.finish().map_err(|error| text(&error))?;
    Ok(())
}

pub fn is_office_archive(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "docx" | "pptx" | "xlsx" | "odt"
    )
}

/// Reads a whole file back, for tests and for the photos of an archive.
pub fn read(path: &Path) -> Option<Vec<u8>> {
    fs::read(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_receives_the_paths_through_the_environment() {
        let command = command(
            &Office::Word,
            Path::new(r"C:\a'; evil.pdf"),
            Path::new(r"C:\out.docx"),
            "docx",
        );
        assert!(
            !command
                .get_args()
                .any(|arg| arg.to_string_lossy().contains("evil"))
        );
        let value = |name: &str| {
            command
                .get_envs()
                .find(|(key, _)| *key == name)
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().to_string())
        };
        assert_eq!(
            value("FILEFORGE_INPUT").as_deref(),
            Some(r"C:\a'; evil.pdf")
        );
        assert_eq!(value("FILEFORGE_FORMAT").as_deref(), Some("16"));
    }

    #[test]
    fn libre_office_reads_a_pdf_as_a_document() {
        let office = Office::LibreOffice(PathBuf::from("soffice"));
        let args = |input: &str, target: &str| {
            command(&office, Path::new(input), Path::new("dir/out"), target)
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>()
        };
        assert!(args("a.PDF", "docx").contains(&"--infilter=writer_pdf_import".to_string()));
        assert!(!args("a.docx", "pdf").contains(&"--infilter=writer_pdf_import".to_string()));
        assert!(args("a.docx", "txt").contains(&"txt:Text".to_string()));
    }

    #[test]
    fn word_only_opens_what_it_knows() {
        assert!(word_opens("PDF"));
        assert!(!word_opens("pptx"));
    }

    #[test]
    fn shrinks_the_photos_of_an_office_file_and_keeps_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.docx");
        let output = dir.path().join("out.docx");
        let mut writer = ZipWriter::new(File::create(&input).unwrap());
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(&[b'x'; 2000]).unwrap();
        writer.start_file("word/media/photo.JPG", options).unwrap();
        writer.write_all(&[7; 500]).unwrap();
        writer.start_file("word/media/big.jpeg", options).unwrap();
        writer.write_all(&[8; 500]).unwrap();
        writer.finish().unwrap();

        let mut shrink = |bytes: &[u8]| {
            (bytes[0] == 7)
                .then(|| vec![1, 2, 3])
                .or_else(|| Some(vec![0; 900]))
        };
        compress_office(&input, &output, &mut shrink).unwrap();

        let mut archive = ZipArchive::new(File::open(&output).unwrap()).unwrap();
        let mut read_entry = |name: &str| {
            let mut bytes = Vec::new();
            archive
                .by_name(name)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            bytes
        };
        assert_eq!(read_entry("word/media/photo.JPG"), [1, 2, 3]);
        assert_eq!(read_entry("word/media/big.jpeg"), [8; 500]);
        assert_eq!(read_entry("word/document.xml"), [b'x'; 2000]);
        assert!(fs::metadata(&output).unwrap().len() < fs::metadata(&input).unwrap().len());
    }
}
