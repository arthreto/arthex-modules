//! What happens to the voice after the model: the deep voice lowered, the sibilants softened and
//! every voice brought to the same level.
//!
//! Lowering resamples the voice, which lowers its pitch and its timbre together like a slowed tape,
//! then WSOLA gives it back most of its length. A phase vocoder would keep the length too, but it
//! makes voices metallic.

// Sample positions and rates stay far below 2^24, where `f32` is exact, and a position turned back
// into an index is never negative.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::f32::consts::PI;

/// How much lower the deep voice is, in semitones.
const DEEP_SEMITONES: f32 = 4.0;
/// The part of the slowing down the deep voice keeps: it speaks a little slower.
const DEEP_SLOWER: f32 = 0.3;
const PEAK: f32 = 0.89;
const SINC_TAPS: isize = 32;
const FRAME: usize = 2048;
const TOLERANCE: usize = 512;

/// Windowed-sinc interpolation: the result is `factor` times longer, so it plays lower.
fn stretched(samples: &[f32], factor: f32) -> Vec<f32> {
    let count = (samples.len() as f32 * factor) as usize;
    let last = samples.len().saturating_sub(1) as isize;
    (0..count)
        .map(|index| {
            let position = index as f32 / factor;
            let base = position.floor();
            let fraction = position - base;
            let base = base as isize;
            (-SINC_TAPS + 1..=SINC_TAPS)
                .map(|tap| {
                    let distance = tap as f32 - fraction;
                    let sinc = if distance.abs() < 1e-6 {
                        1.0
                    } else {
                        (PI * distance).sin() / (PI * distance)
                    };
                    let window = 0.5 + 0.5 * (PI * distance / SINC_TAPS as f32).cos();
                    samples[(base + tap).clamp(0, last) as usize] * sinc * window
                })
                .sum()
        })
        .collect()
}

/// Changes the length by `factor` without changing the pitch: each frame is taken where it
/// continues the previous one best, near where it should be.
fn wsola(samples: &[f32], factor: f32) -> Vec<f32> {
    let hop = FRAME / 2;
    let window: Vec<f32> = (0..FRAME)
        .map(|index| 0.5 - 0.5 * (2.0 * PI * index as f32 / FRAME as f32).cos())
        .collect();
    let mut padded = vec![0.0; TOLERANCE];
    padded.extend_from_slice(samples);
    padded.resize(
        padded.len() + FRAME + TOLERANCE + hop + (hop as f32 / factor) as usize + 1,
        0.0,
    );
    let length = (samples.len() as f32 * factor) as usize;
    let frames = length / hop + 1;
    let mut out = vec![0.0; frames * hop + FRAME];
    let mut previous = TOLERANCE;
    for frame in 0..frames {
        let nominal = TOLERANCE + (frame as f32 * hop as f32 / factor) as usize;
        let chosen = if frame == 0 {
            nominal
        } else {
            let natural = &padded[previous + hop..previous + hop + FRAME];
            let start = nominal - TOLERANCE;
            (start..=nominal + TOLERANCE)
                .map(|candidate| {
                    // Every other sample is enough to find where the waves line up.
                    let score: f32 = padded[candidate..candidate + FRAME]
                        .iter()
                        .zip(natural)
                        .step_by(2)
                        .map(|(a, b)| a * b)
                        .sum();
                    (candidate, score)
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map_or(nominal, |(candidate, _)| candidate)
        };
        for (index, weight) in window.iter().enumerate() {
            out[frame * hop + index] += padded[chosen + index] * weight;
        }
        previous = chosen;
    }
    out.truncate(length);
    out
}

/// Gives the chest of the voice a little more weight.
fn low_shelf(samples: &mut [f32], rate: u32, gain_db: f32, corner: f32) {
    let alpha = (-2.0 * PI * corner / rate as f32).exp();
    let extra = 10_f32.powf(gain_db / 20.0) - 1.0;
    let mut low = 0.0;
    for sample in samples {
        low = (1.0 - alpha) * *sample + alpha * low;
        *sample += extra * low;
    }
}

/// A gentle low-pass: the sibilants otherwise set the peak and sound harsh once the voice is
/// brought up.
fn soften(samples: &mut [f32], rate: u32, corner: f32) {
    let omega = 2.0 * PI * corner / rate as f32;
    let alpha = omega.sin() / 2.0 * std::f32::consts::SQRT_2;
    let cos = omega.cos();
    let a0 = 1.0 + alpha;
    let b0 = (1.0 - cos) / 2.0 / a0;
    let b1 = (1.0 - cos) / a0;
    let a1 = -2.0 * cos / a0;
    let a2 = (1.0 - alpha) / a0;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
    for sample in samples {
        let x = *sample;
        let y = b0 * x + b1 * x1 + b0 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, x, y1, y);
        *sample = y;
    }
}

/// The deep voice, from the voice the model made with its style.
pub fn deepen(samples: &[f32], rate: u32) -> Vec<f32> {
    let factor = 2_f32.powf(DEEP_SEMITONES / 12.0);
    let mut lowered = wsola(
        &stretched(samples, factor),
        (1.0 + (factor - 1.0) * DEEP_SLOWER) / factor,
    );
    low_shelf(&mut lowered, rate, 3.0, 180.0);
    lowered
}

/// Every voice comes out at the same level, which the model does not give.
pub fn finish(samples: &mut [f32], rate: u32) {
    soften(samples, rate, 11_000.0);
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, value| peak.max(value.abs()));
    if peak > 1e-4 {
        for sample in samples {
            *sample *= PEAK / peak;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44_100;

    fn tone(frequency: f32, seconds: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize)
            .map(|index| 0.3 * (2.0 * PI * frequency * index as f32 / RATE as f32).sin())
            .collect()
    }

    /// Counts upward zero crossings, for the pitch of a steady tone.
    fn frequency(samples: &[f32]) -> f32 {
        let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
        let crossings = middle
            .windows(2)
            .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
            .count();
        crossings as f32 * RATE as f32 / middle.len() as f32
    }

    #[test]
    fn the_deep_voice_is_four_semitones_lower_and_a_little_slower() {
        let source = tone(200.0, 1.0);
        let deep = deepen(&source, RATE);
        let expected = 200.0 / 2_f32.powf(4.0 / 12.0);
        assert!(
            (frequency(&deep) - expected).abs() < 3.0,
            "{}",
            frequency(&deep)
        );
        let ratio = deep.len() as f32 / source.len() as f32;
        assert!((1.06..1.1).contains(&ratio), "{ratio}");
    }

    #[test]
    fn wsola_keeps_the_pitch() {
        let longer = wsola(&tone(150.0, 1.0), 1.5);
        assert_eq!(longer.len(), RATE as usize * 3 / 2);
        assert!((frequency(&longer) - 150.0).abs() < 2.0);
    }

    #[test]
    fn every_voice_ends_at_the_same_peak_without_clicks() {
        let mut quiet = tone(220.0, 0.5);
        finish(&mut quiet, RATE);
        let peak = quiet
            .iter()
            .fold(0.0_f32, |peak, value| peak.max(value.abs()));
        assert!((peak - PEAK).abs() < 0.01);
        assert!(quiet.windows(2).all(|pair| (pair[1] - pair[0]).abs() < 0.1));
    }

    #[test]
    fn silence_stays_silent() {
        let mut silence = vec![0.0; 1000];
        finish(&mut silence, RATE);
        assert!(silence.iter().all(|value| *value == 0.0));
        assert!(deepen(&silence, RATE).iter().all(|value| *value == 0.0));
    }
}
