//! The voice styles of the model, as `voice.bin` holds them, and the deep voice the module adds.
//!
//! The file starts with six little-endian `i64`: the count and shape of the main styles, then the
//! count and shape of the duration styles, followed by both as `f32`. Every row of a style is a
//! unit vector.

/// The voice the module adds after the ten of the model.
pub const DEEP: usize = 10;
/// The deepest man of the model, which the deep voice starts from.
const BASE: usize = 6;
const WOMEN: std::ops::Range<usize> = 0..5;
const MEN: std::ops::Range<usize> = 5..10;
const HEADER: usize = 6 * 8;

#[derive(Debug, PartialEq)]
pub struct Styles {
    count: usize,
    main: (usize, usize),
    duration: (usize, usize),
    main_values: Vec<f32>,
    duration_values: Vec<f32>,
}

fn dimension(bytes: &[u8], index: usize) -> Result<usize, String> {
    let start = index * 8;
    let raw: [u8; 8] = bytes
        .get(start..start + 8)
        .and_then(|slice| slice.try_into().ok())
        .ok_or("voices: too short")?;
    usize::try_from(i64::from_le_bytes(raw)).map_err(|_| "voices: bad shape".to_string())
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    let (chunks, _) = bytes.as_chunks::<4>();
    chunks
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect()
}

/// `base + amount × (men − women)` for one style, each row scaled back to a unit vector.
fn pushed(values: &[f32], size: usize, amount: f32) -> Vec<f32> {
    let style = |index: usize| &values[index * size..(index + 1) * size];
    let average = |range: std::ops::Range<usize>| {
        let count = f32::from(u16::try_from(range.len()).unwrap_or(u16::MAX));
        let mut sum = vec![0.0; size];
        for index in range {
            for (total, value) in sum.iter_mut().zip(style(index)) {
                *total += value / count;
            }
        }
        sum
    };
    let (men, women) = (average(MEN), average(WOMEN));
    style(BASE)
        .iter()
        .zip(men.iter().zip(&women))
        .map(|(base, (man, woman))| base + amount * (man - woman))
        .collect()
}

fn unit_rows(values: &mut [f32], row: usize) {
    for chunk in values.chunks_mut(row) {
        let length = chunk.iter().map(|value| value * value).sum::<f32>().sqrt();
        if length > 0.0 {
            for value in chunk {
                *value /= length;
            }
        }
    }
}

impl Styles {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let count = dimension(bytes, 0)?;
        let main = (dimension(bytes, 1)?, dimension(bytes, 2)?);
        let duration = (dimension(bytes, 4)?, dimension(bytes, 5)?);
        if dimension(bytes, 3)? != count || count < MEN.end {
            return Err("voices: unexpected count".to_string());
        }
        let main_bytes = count * main.0 * main.1 * 4;
        let duration_bytes = count * duration.0 * duration.1 * 4;
        if bytes.len() != HEADER + main_bytes + duration_bytes {
            return Err("voices: unexpected size".to_string());
        }
        Ok(Self {
            count,
            main,
            duration,
            main_values: floats(&bytes[HEADER..HEADER + main_bytes]),
            duration_values: floats(&bytes[HEADER + main_bytes..]),
        })
    }

    /// The same styles, with the deep voice added as number `DEEP`.
    pub fn with_deep_voice(mut self) -> Self {
        let main_size = self.main.0 * self.main.1;
        let duration_size = self.duration.0 * self.duration.1;
        let mut main = pushed(&self.main_values, main_size, 1.0);
        let mut duration = pushed(&self.duration_values, duration_size, 0.5);
        unit_rows(&mut main, self.main.1);
        unit_rows(&mut duration, self.duration.1);
        self.main_values.truncate(DEEP * main_size);
        self.duration_values.truncate(DEEP * duration_size);
        self.main_values.extend(main);
        self.duration_values.extend(duration);
        self.count = DEEP + 1;
        self
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let shape = [
            self.count,
            self.main.0,
            self.main.1,
            self.count,
            self.duration.0,
            self.duration.1,
        ];
        let mut bytes: Vec<u8> = shape
            .iter()
            .flat_map(|value| i64::try_from(*value).unwrap_or(0).to_le_bytes())
            .collect();
        for value in self.main_values.iter().chain(&self.duration_values) {
            bytes.extend(value.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten voices of 2 × 2 main values and 1 × 2 duration values.
    fn sample() -> Vec<u8> {
        let mut bytes: Vec<u8> = [10_i64, 2, 2, 10, 1, 2]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        for voice in 0..10_u8 {
            let lean = if voice < 5 { 0.6_f32 } else { 0.8 };
            let rest = (1.0 - lean * lean).sqrt();
            for value in [lean, rest, rest, lean] {
                bytes.extend(value.to_le_bytes());
            }
        }
        for _ in 0..10 {
            bytes.extend(1.0_f32.to_le_bytes());
            bytes.extend(0.0_f32.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn reads_and_writes_the_same_file() {
        let bytes = sample();
        assert_eq!(Styles::parse(&bytes).unwrap().to_bytes(), bytes);
    }

    #[test]
    fn adds_a_deep_voice_further_from_the_women() {
        let styles = Styles::parse(&sample()).unwrap().with_deep_voice();
        let bytes = styles.to_bytes();
        assert_eq!(dimension(&bytes, 0).unwrap(), 11);
        let deep = &styles.main_values[DEEP * 4..DEEP * 4 + 4];
        assert!(deep[0] > 0.8, "{deep:?}");
        for row in deep.chunks(2) {
            let length: f32 = row.iter().map(|value| value * value).sum();
            assert!((length - 1.0).abs() < 1e-5);
        }
        assert_eq!(Styles::parse(&bytes).unwrap(), styles);
    }

    #[test]
    fn refuses_another_file() {
        assert!(Styles::parse(b"hello").is_err());
        let mut bytes = sample();
        bytes.pop();
        assert!(Styles::parse(&bytes).is_err());
    }
}
