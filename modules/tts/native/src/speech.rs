//! Turns a text into a voice with the model, in the program itself: the text reaches the model as
//! UTF-8, which the command line of sherpa-onnx does not give on Windows.

use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread,
};

use serde::Deserialize;
use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsModelConfig,
    OfflineTtsSupertonicModelConfig,
};

use crate::{model::VOICES, sound, voices::DEEP};

/// Long enough for a chapter, short enough to stay in memory.
const MAX_CHARACTERS: usize = 20_000;
/// More steps sound cleaner; past this, the difference is no longer heard.
const STEPS: i32 = 8;
/// The deep voice speaks slower than the others at the same speed.
const DEEP_PACE: f32 = 0.85;
const SPEEDS: std::ops::RangeInclusive<f32> = 0.5..=2.0;

#[derive(Deserialize)]
pub struct Request {
    text: String,
    language: String,
    voice: usize,
    speed: f32,
}

impl Request {
    /// The text the model reads, or why it cannot.
    pub fn checked(mut self) -> Result<Self, String> {
        if !matches!(self.language.as_str(), "fr" | "en") {
            return Err("language".to_string());
        }
        if self.voice > DEEP || !SPEEDS.contains(&self.speed) {
            return Err("voice".to_string());
        }
        // A NUL would cut the text where the model receives it.
        self.text = self
            .text
            .chars()
            .filter(|character| *character != '\0')
            .take(MAX_CHARACTERS)
            .collect::<String>()
            .trim()
            .to_string();
        if self.text.is_empty() {
            return Err("empty".to_string());
        }
        Ok(self)
    }
}

pub struct Spoken {
    pub samples: Vec<f32>,
    pub rate: u32,
}

pub struct Engine {
    tts: OfflineTts,
}

impl Engine {
    pub fn load(folder: &Path) -> Option<Self> {
        let path = |name: &str| Some(folder.join(name).to_string_lossy().into_owned());
        let threads = thread::available_parallelism().map_or(4, |count| count.get().min(6));
        let config = OfflineTtsConfig {
            model: OfflineTtsModelConfig {
                supertonic: OfflineTtsSupertonicModelConfig {
                    duration_predictor: path("duration_predictor.int8.onnx"),
                    text_encoder: path("text_encoder.int8.onnx"),
                    vector_estimator: path("vector_estimator.int8.onnx"),
                    vocoder: path("vocoder.int8.onnx"),
                    tts_json: path("tts.json"),
                    unicode_indexer: path("unicode_indexer.bin"),
                    voice_style: path(VOICES),
                },
                num_threads: i32::try_from(threads).unwrap_or(4),
                ..OfflineTtsModelConfig::default()
            },
            max_num_sentences: 1,
            silence_scale: 0.2,
            ..OfflineTtsConfig::default()
        };
        OfflineTts::create(&config).map(|tts| Self { tts })
    }

    /// The voice of `request`, or nothing when it was cancelled or failed. `percent` follows the
    /// model, which reports its progress sentence by sentence.
    pub fn speak(
        &self,
        request: &Request,
        percent: &Arc<AtomicU8>,
        cancelled: &Arc<AtomicBool>,
    ) -> Option<Spoken> {
        let deep = request.voice == DEEP;
        let config = GenerationConfig {
            sid: i32::try_from(request.voice).ok()?,
            speed: if deep {
                request.speed * DEEP_PACE
            } else {
                request.speed
            },
            num_steps: STEPS,
            extra: Some(HashMap::from([(
                "lang".to_string(),
                serde_json::Value::from(request.language.as_str()),
            )])),
            ..GenerationConfig::default()
        };
        let (progress, stop) = (Arc::clone(percent), Arc::clone(cancelled));
        let audio = self.tts.generate_with_config(
            &request.text,
            &config,
            Some(move |_: &[f32], done: f32| {
                // The model is most of the work: the rest is left for the sound.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                progress.store((done.clamp(0.0, 1.0) * 95.0) as u8, Ordering::Relaxed);
                !stop.load(Ordering::Relaxed)
            }),
        )?;
        if cancelled.load(Ordering::Relaxed) {
            return None;
        }
        let rate = u32::try_from(audio.sample_rate()).ok()?;
        let mut samples = if deep {
            sound::deepen(audio.samples(), rate)
        } else {
            audio.samples().to_vec()
        };
        sound::finish(&mut samples, rate);
        Some(Spoken { samples, rate })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(text: &str, language: &str, voice: usize, speed: f32) -> Request {
        Request {
            text: text.to_string(),
            language: language.to_string(),
            voice,
            speed,
        }
    }

    #[test]
    fn accepts_the_choices_of_the_interface() {
        let checked = request("  Écoute bien.\0 ", "fr", DEEP, 1.2)
            .checked()
            .unwrap();
        assert_eq!(checked.text, "Écoute bien.");
        assert!(request("Hello", "en", 0, 0.8).checked().is_ok());
    }

    #[test]
    fn refuses_what_it_cannot_say() {
        assert_eq!(
            request(" \n ", "fr", 0, 1.0).checked().err().unwrap(),
            "empty"
        );
        assert_eq!(
            request("Hola", "es", 0, 1.0).checked().err().unwrap(),
            "language"
        );
        assert_eq!(
            request("Hi", "en", DEEP + 1, 1.0).checked().err().unwrap(),
            "voice"
        );
        assert_eq!(
            request("Hi", "en", 0, 3.0).checked().err().unwrap(),
            "voice"
        );
    }

    #[test]
    fn keeps_a_long_text_within_bounds() {
        let long = "a".repeat(MAX_CHARACTERS + 10);
        assert_eq!(
            request(&long, "en", 0, 1.0).checked().unwrap().text.len(),
            MAX_CHARACTERS
        );
    }
}
