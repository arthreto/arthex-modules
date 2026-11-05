use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use windows::{
    Globalization::Language,
    Graphics::Imaging::{
        BitmapAlphaMode, BitmapDecoder, BitmapEncoder, BitmapInterpolationMode, BitmapPixelFormat,
        BitmapTransform, ColorManagementMode, ExifOrientationMode, SoftwareBitmap,
    },
    Media::Ocr::OcrEngine,
    Storage::{
        FileAccessMode, StorageFile,
        Streams::{Buffer, DataReader, DataWriter, InMemoryRandomAccessStream},
    },
    core::{HSTRING, Result as WinResult},
};

use crate::bitmap::{Bitmap, reading_scale};

/// What the interface shows: a message of the program is limited to 4 MB.
const SHOWN_LONGEST_SIDE: u32 = 2000;
/// Told apart by the interface, which explains it: the language must be installed in Windows.
pub const NO_LANGUAGE: &str = "no-language";

#[derive(Serialize)]
pub struct ReadWord {
    text: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    line: usize,
}

#[derive(Serialize)]
pub struct ReadLanguage {
    tag: String,
    name: String,
}

#[derive(Serialize)]
pub struct Languages {
    languages: Vec<ReadLanguage>,
    current: Option<String>,
}

fn message(error: &windows::core::Error) -> String {
    error.message()
}

pub fn languages() -> Result<Languages, String> {
    let read = || -> WinResult<Languages> {
        let mut languages = Vec::new();
        for language in OcrEngine::AvailableRecognizerLanguages()? {
            languages.push(ReadLanguage {
                tag: language.LanguageTag()?.to_string(),
                name: language.DisplayName()?.to_string(),
            });
        }
        let current = OcrEngine::TryCreateFromUserProfileLanguages()
            .and_then(|engine| engine.RecognizerLanguage())
            .and_then(|language| language.LanguageTag())
            .map(|tag| tag.to_string())
            .ok();
        Ok(Languages { languages, current })
    };
    read().map_err(|error| message(&error))
}

fn engine(language: Option<&str>) -> Result<OcrEngine, String> {
    let created = match language {
        Some(tag) => Language::CreateLanguage(&HSTRING::from(tag))
            .and_then(|language| OcrEngine::TryCreateFromLanguage(&language)),
        None => OcrEngine::TryCreateFromUserProfileLanguages(),
    };
    // Windows answers with nothing, not an error, when no installed language can read.
    created.map_err(|_| NO_LANGUAGE.to_string())
}

fn to_software_bitmap(image: &Bitmap) -> WinResult<SoftwareBitmap> {
    let software = SoftwareBitmap::CreateWithAlpha(
        BitmapPixelFormat::Bgra8,
        image.width.cast_signed(),
        image.height.cast_signed(),
        BitmapAlphaMode::Premultiplied,
    )?;
    let writer = DataWriter::new()?;
    writer.WriteBytes(&image.pixels)?;
    software.CopyFromBuffer(&writer.DetachBuffer()?)?;
    Ok(software)
}

fn pixels_of(software: &SoftwareBitmap) -> WinResult<Vec<u8>> {
    let (width, height) = (
        software.PixelWidth()?.cast_unsigned(),
        software.PixelHeight()?.cast_unsigned(),
    );
    let buffer = Buffer::Create(width * height * 4)?;
    software.CopyToBuffer(&buffer)?;
    let reader = DataReader::FromBuffer(&buffer)?;
    let mut pixels = vec![0; buffer.Length()? as usize];
    reader.ReadBytes(&mut pixels)?;
    Ok(pixels)
}

/// Whatever Windows can decode, turned the way the photo was taken.
pub fn load(path: &str) -> Result<Bitmap, String> {
    let read = || -> WinResult<Bitmap> {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path))?.join()?;
        let stream = file.OpenAsync(FileAccessMode::Read)?.join()?;
        let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
        let software = decoder
            .GetSoftwareBitmapTransformedAsync(
                BitmapPixelFormat::Bgra8,
                BitmapAlphaMode::Premultiplied,
                &BitmapTransform::new()?,
                ExifOrientationMode::RespectExifOrientation,
                ColorManagementMode::ColorManageToSRgb,
            )?
            .join()?;
        Ok(Bitmap::over_white(
            software.PixelWidth()?.cast_unsigned(),
            software.PixelHeight()?.cast_unsigned(),
            pixels_of(&software)?,
        ))
    };
    read().map_err(|error| message(&error))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn shown_size(width: u32, height: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= SHOWN_LONGEST_SIDE {
        return (width, height);
    }
    let ratio = SHOWN_LONGEST_SIDE as f32 / longest as f32;
    (
        ((width as f32 * ratio) as u32).max(1),
        ((height as f32 * ratio) as u32).max(1),
    )
}

/// The image as a JPEG for the tool's `<img>`: text is read from the original, never from this.
pub fn data_url(image: &Bitmap) -> Result<String, String> {
    let encode = || -> WinResult<Vec<u8>> {
        let stream = InMemoryRandomAccessStream::new()?;
        let encoder =
            BitmapEncoder::CreateAsync(BitmapEncoder::JpegEncoderId()?, &stream)?.join()?;
        encoder.SetSoftwareBitmap(&to_software_bitmap(image)?)?;
        let (width, height) = shown_size(image.width, image.height);
        let transform = encoder.BitmapTransform()?;
        transform.SetScaledWidth(width)?;
        transform.SetScaledHeight(height)?;
        transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
        encoder.FlushAsync()?.join()?;
        let length = u32::try_from(stream.Size()?).unwrap_or(u32::MAX);
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(length)?.join()?;
        let mut bytes = vec![0; length as usize];
        reader.ReadBytes(&mut bytes)?;
        Ok(bytes)
    };
    encode()
        .map(|bytes| format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
        .map_err(|error| message(&error))
}

/// Reads every word of an image, with its place in the image.
#[allow(clippy::cast_precision_loss)]
pub fn read(image: &Bitmap, language: Option<&str>) -> Result<Vec<ReadWord>, String> {
    let engine = engine(language)?;
    let scale = reading_scale(image.width, image.height);
    let enlarged = image.enlarged(scale);
    let found = || -> WinResult<Vec<ReadWord>> {
        let software = to_software_bitmap(&enlarged)?;
        let result = engine.RecognizeAsync(&software)?.join()?;
        let mut words = Vec::new();
        for (line, content) in result.Lines()?.into_iter().enumerate() {
            for word in content.Words()? {
                let place = word.BoundingRect()?;
                let factor = scale as f32;
                words.push(ReadWord {
                    text: word.Text()?.to_string(),
                    x: place.X / factor,
                    y: place.Y / factor,
                    w: place.Width / factor,
                    h: place.Height / factor,
                    line,
                });
            }
        }
        Ok(words)
    };
    found().map_err(|error| message(&error))
}
