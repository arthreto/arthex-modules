use crate::{
    moment::{Moment, day_of, hour_and_minute, month_and_day},
    store::{Item, Kind},
};

/// The texts of the HUD. It runs with the interface closed, so they are written here.
#[derive(Clone, Copy)]
pub struct Texts {
    french: bool,
}

/// What a reminder says, its progress when it has one, and the colour of its pictogram.
#[derive(Debug, PartialEq)]
pub struct Reminder {
    pub id: String,
    pub title: String,
    pub when: String,
    pub progress: Option<f32>,
    pub tint: String,
}

impl Texts {
    pub fn for_locale(locale: &str) -> Self {
        Self {
            french: locale.starts_with("fr"),
        }
    }

    pub const fn later(self) -> &'static str {
        if self.french { "Plus tard" } else { "Later" }
    }

    fn time(self, moment: Moment) -> String {
        let (hour, minute) = hour_and_minute(moment);
        match (self.french, minute) {
            (true, 0) => format!("{hour} h"),
            (true, _) => format!("{hour} h {minute:02}"),
            (false, _) => format!("{hour}:{minute:02}"),
        }
    }

    fn span_until(self, minutes: i64) -> String {
        if minutes >= 1440 {
            let days = minutes / 1440;
            match (self.french, days) {
                (true, 1) => "1 jour".to_string(),
                (true, _) => format!("{days} jours"),
                (false, 1) => "1 day".to_string(),
                (false, _) => format!("{days} days"),
            }
        } else if minutes >= 60 {
            format!("{} h", minutes / 60)
        } else {
            format!("{} min", minutes.max(1))
        }
    }

    fn event_when(self, start: Moment, end: Moment, now: Moment) -> String {
        let times = if self.french {
            format!("{} à {}", self.time(start), self.time(end))
        } else {
            format!("{} to {}", self.time(start), self.time(end))
        };
        let left = start - now;
        let lead = if left <= 0 {
            (if self.french { "Maintenant" } else { "Now" }).to_string()
        } else if self.french {
            format!("Dans {}", self.span_until(left))
        } else {
            format!("In {}", self.span_until(left))
        };
        format!("{lead} · {times}")
    }

    fn range_when(self, end: Moment, now: Moment) -> String {
        let left = self.span_until(end - now);
        let time = self.time(end);
        let (month, day) = month_and_day(end);
        match (self.french, day_of(end) - day_of(now)) {
            (true, 0) => format!("Reste {left} · fin à {time}"),
            (true, 1) => format!("Reste {left} · fin demain à {time}"),
            (true, _) => format!("Reste {left} · fin le {day:02}/{month:02} à {time}"),
            (false, 0) => format!("{left} left · ends at {time}"),
            (false, 1) => format!("{left} left · ends tomorrow at {time}"),
            (false, _) => format!("{left} left · ends on {month:02}/{day:02} at {time}"),
        }
    }

    pub fn reminder(self, item: &Item, now: Moment) -> Option<Reminder> {
        let (start, end) = (item.starts()?, item.ends()?);
        let (when, progress) = match item.kind {
            Kind::Event => (self.event_when(start, end, now), None),
            Kind::Range => (self.range_when(end, now), Some(progress(start, end, now))),
        };
        Some(Reminder {
            id: item.id.clone(),
            title: item.title.clone(),
            when,
            progress,
            tint: item.tint.clone(),
        })
    }
}

#[allow(clippy::cast_precision_loss)]
fn progress(start: Moment, end: Moment, now: Moment) -> f32 {
    (((now - start) as f32) / ((end - start) as f32)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moment::parse;

    fn item(kind: Kind, start: &str, end: &str) -> Item {
        Item {
            id: "a".to_string(),
            kind,
            title: "Report".to_string(),
            tint: "orange".to_string(),
            start: start.to_string(),
            end: end.to_string(),
            reminder: Some(10),
        }
    }

    fn when(locale: &str, item: &Item, now: &str) -> String {
        Texts::for_locale(locale)
            .reminder(item, parse(now).unwrap())
            .unwrap()
            .when
    }

    #[test]
    fn an_event_says_how_long_is_left() {
        let event = item(Kind::Event, "2026-11-06T10:00", "2026-11-06T10:30");
        assert_eq!(
            when("fr", &event, "2026-11-06T09:50"),
            "Dans 10 min · 10 h à 10 h 30"
        );
        assert_eq!(
            when("en", &event, "2026-11-06T09:50"),
            "In 10 min · 10:00 to 10:30"
        );
    }

    #[test]
    fn an_event_that_starts_says_so() {
        let event = item(Kind::Event, "2026-11-06T10:00", "2026-11-06T10:30");
        assert_eq!(
            when("fr", &event, "2026-11-06T10:00"),
            "Maintenant · 10 h à 10 h 30"
        );
        assert_eq!(
            when("en", &event, "2026-11-06T10:05"),
            "Now · 10:00 to 10:30"
        );
    }

    #[test]
    fn an_event_far_away_counts_in_days() {
        let event = item(Kind::Event, "2026-11-08T10:00", "2026-11-08T10:30");
        assert_eq!(
            when("fr", &event, "2026-11-07T10:00"),
            "Dans 1 jour · 10 h à 10 h 30"
        );
        assert_eq!(
            when("en", &event, "2026-11-06T10:00"),
            "In 2 days · 10:00 to 10:30"
        );
    }

    #[test]
    fn a_deadline_says_what_is_left_and_when_it_ends() {
        let range = item(Kind::Range, "2026-11-02T09:00", "2026-11-13T18:00");
        assert_eq!(
            when("fr", &range, "2026-11-12T18:00"),
            "Reste 1 jour · fin demain à 18 h"
        );
        assert_eq!(
            when("en", &range, "2026-11-12T18:00"),
            "1 day left · ends tomorrow at 18:00"
        );
        assert_eq!(
            when("fr", &range, "2026-11-13T15:00"),
            "Reste 3 h · fin à 18 h"
        );
        assert_eq!(
            when("fr", &range, "2026-11-10T18:00"),
            "Reste 3 jours · fin le 13/11 à 18 h"
        );
    }

    #[test]
    fn only_a_deadline_has_progress() {
        let texts = Texts::for_locale("en");
        let range = item(Kind::Range, "2026-11-01T00:00", "2026-11-03T00:00");
        let event = item(Kind::Event, "2026-11-06T10:00", "2026-11-06T10:30");
        let now = parse("2026-11-02T00:00").unwrap();
        assert_eq!(texts.reminder(&range, now).unwrap().progress, Some(0.5));
        assert_eq!(texts.reminder(&event, now).unwrap().progress, None);
    }

    #[test]
    fn progress_stays_in_its_bounds() {
        assert_eq!(progress(10, 20, 5), 0.0);
        assert_eq!(progress(10, 20, 25), 1.0);
    }
}
