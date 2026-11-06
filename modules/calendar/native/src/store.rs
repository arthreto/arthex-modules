use std::{collections::HashMap, fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::moment::{self, Moment};

const MAX_TITLE: usize = 200;
const MAX_ID: usize = 64;
const MAX_ITEMS: usize = 5000;
const TINTS: [&str; 6] = ["sky", "violet", "green", "orange", "pink", "red"];
/// How long before an event or the end of a deadline a reminder can come, in minutes.
const REMINDERS: [u32; 6] = [0, 10, 60, 1440, 4320, 10_080];
const SNOOZE_EVENT: i64 = 10;
const SNOOZE_DEADLINE: i64 = 1440;
const FILE: &str = "calendar.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Event,
    Range,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub kind: Kind,
    pub title: String,
    pub tint: String,
    /// `YYYY-MM-DDTHH:MM`, local.
    pub start: String,
    pub end: String,
    /// Minutes before the event, or before the end of the deadline. Nothing means no reminder.
    pub reminder: Option<u32>,
}

impl Item {
    /// What comes from the interface is checked here, before it is kept or acted upon.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty()
            || self.id.len() > MAX_ID
            || !self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err("invalid id".to_string());
        }
        let title = self.title.trim();
        if title.is_empty() || title.chars().count() > MAX_TITLE {
            return Err("invalid title".to_string());
        }
        if !TINTS.contains(&self.tint.as_str()) {
            return Err("invalid colour".to_string());
        }
        let (Some(start), Some(end)) = (self.starts(), self.ends()) else {
            return Err("invalid date".to_string());
        };
        if end <= start {
            return Err("the end comes before the start".to_string());
        }
        if self.kind == Kind::Event && moment::day_of(start) != moment::day_of(end) {
            return Err("an event stays on one day".to_string());
        }
        if self
            .reminder
            .is_some_and(|minutes| !REMINDERS.contains(&minutes))
        {
            return Err("invalid reminder".to_string());
        }
        Ok(())
    }

    pub fn starts(&self) -> Option<Moment> {
        moment::parse(&self.start)
    }

    pub fn ends(&self) -> Option<Moment> {
        moment::parse(&self.end)
    }

    /// When its reminder is due: before an event starts, or before a deadline ends.
    fn due(&self) -> Option<Moment> {
        let anchor = match self.kind {
            Kind::Event => self.starts()?,
            Kind::Range => self.ends()?,
        };
        Some(anchor - i64::from(self.reminder?))
    }
}

/// Everything kept on disk.
#[derive(Default, Serialize, Deserialize)]
pub struct Store {
    pub items: Vec<Item>,
    /// The moment each reminder was due when it last showed: moving the date shows it again.
    #[serde(default)]
    shown: HashMap<String, Moment>,
    /// When a reminder comes back after "Later".
    #[serde(default)]
    snoozed: HashMap<String, Moment>,
}

impl Store {
    pub fn load(folder: &Path) -> Self {
        fs::read(folder.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// In one rename, so that a crash in the middle never loses what was there.
    pub fn save(&self, folder: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        let temporary = folder.join(format!("{FILE}.tmp"));
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, folder.join(FILE)).map_err(|error| error.to_string())
    }

    pub fn put(&mut self, item: Item) -> Result<(), String> {
        item.validate()?;
        self.snoozed.remove(&item.id);
        if let Some(existing) = self.items.iter_mut().find(|other| other.id == item.id) {
            *existing = item;
        } else if self.items.len() < MAX_ITEMS {
            self.items.push(item);
        } else {
            return Err("too many items".to_string());
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) {
        self.items.retain(|item| item.id != id);
        self.shown.remove(id);
        self.snoozed.remove(id);
    }

    /// When an item's reminder must show next, or nothing when it has shown already.
    fn next_of(&self, item: &Item) -> Option<Moment> {
        if let Some(until) = self.snoozed.get(&item.id) {
            return Some(*until);
        }
        let due = item.due()?;
        (self.shown.get(&item.id) != Some(&due)).then_some(due)
    }

    /// Takes the reminders to show now, each once. After the event or the deadline, one that
    /// was missed with the computer off is of no use.
    pub fn take_due(&mut self, now: Moment) -> Vec<Item> {
        let due: Vec<Item> = self
            .items
            .iter()
            .filter(|item| {
                self.next_of(item).is_some_and(|next| next <= now)
                    && item.ends().is_some_and(|end| end > now)
            })
            .cloned()
            .collect();
        for item in &due {
            self.snoozed.remove(&item.id);
            if let Some(moment) = item.due() {
                self.shown.insert(item.id.clone(), moment);
            }
        }
        due
    }

    /// The next time something must show, to wait for it without looking.
    pub fn next_wake(&self, now: Moment) -> Option<Moment> {
        self.items
            .iter()
            .filter(|item| item.ends().is_some_and(|end| end > now))
            .filter_map(|item| self.next_of(item))
            .filter(|next| *next > now)
            .min()
    }

    /// "Later": ten minutes for an event, a day for a deadline, never past its end.
    pub fn snooze(&mut self, id: &str, now: Moment) {
        let Some(item) = self.items.iter().find(|item| item.id == id) else {
            return;
        };
        let wait = match item.kind {
            Kind::Event => SNOOZE_EVENT,
            Kind::Range => SNOOZE_DEADLINE,
        };
        if item.ends().is_some_and(|end| now + wait < end) {
            self.snoozed.insert(id.to_string(), now + wait);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, start: &str, end: &str, reminder: Option<u32>) -> Item {
        Item {
            id: id.to_string(),
            kind: Kind::Event,
            title: "Meeting".to_string(),
            tint: "sky".to_string(),
            start: start.to_string(),
            end: end.to_string(),
            reminder,
        }
    }

    fn deadline(id: &str, start: &str, end: &str, reminder: Option<u32>) -> Item {
        Item {
            kind: Kind::Range,
            ..event(id, start, end, reminder)
        }
    }

    fn at(text: &str) -> Moment {
        moment::parse(text).unwrap()
    }

    fn with(items: Vec<Item>) -> Store {
        Store {
            items,
            ..Store::default()
        }
    }

    fn ids(items: &[Item]) -> Vec<&str> {
        items.iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn accepts_a_good_item() {
        let item = event("a-1", "2026-11-06T10:00", "2026-11-06T10:30", Some(10));
        assert_eq!(item.validate(), Ok(()));
    }

    #[test]
    fn refuses_what_is_not_valid() {
        let good = event("a", "2026-11-06T10:00", "2026-11-06T10:30", Some(10));
        let cases = [
            Item {
                id: "../a".to_string(),
                ..good.clone()
            },
            Item {
                title: "  ".to_string(),
                ..good.clone()
            },
            Item {
                title: "x".repeat(201),
                ..good.clone()
            },
            Item {
                tint: "#fff".to_string(),
                ..good.clone()
            },
            Item {
                start: "soon".to_string(),
                ..good.clone()
            },
            Item {
                end: "2026-11-06T09:00".to_string(),
                ..good.clone()
            },
            Item {
                end: "2026-11-07T09:00".to_string(),
                ..good.clone()
            },
            Item {
                reminder: Some(7),
                ..good
            },
        ];
        for case in cases {
            assert!(case.validate().is_err(), "{case:?}");
        }
    }

    #[test]
    fn a_deadline_can_cross_days() {
        let item = deadline("a", "2026-11-02T09:00", "2026-11-13T18:00", Some(1440));
        assert_eq!(item.validate(), Ok(()));
    }

    #[test]
    fn an_event_shows_before_it_starts_and_once() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T10:30",
            Some(10),
        )]);
        assert_eq!(store.take_due(at("2026-11-06T09:49")), Vec::<Item>::new());
        assert_eq!(ids(&store.take_due(at("2026-11-06T09:50"))), ["a"]);
        assert_eq!(store.take_due(at("2026-11-06T09:51")), Vec::<Item>::new());
    }

    #[test]
    fn a_deadline_shows_before_it_ends() {
        let mut store = with(vec![deadline(
            "a",
            "2026-11-02T09:00",
            "2026-11-13T18:00",
            Some(1440),
        )]);
        assert_eq!(store.take_due(at("2026-11-12T17:59")), Vec::<Item>::new());
        assert_eq!(ids(&store.take_due(at("2026-11-12T18:00"))), ["a"]);
    }

    #[test]
    fn no_reminder_never_shows() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T10:30",
            None,
        )]);
        assert_eq!(store.take_due(at("2026-11-06T10:15")), Vec::<Item>::new());
        assert_eq!(store.next_wake(at("2026-11-06T09:00")), None);
    }

    #[test]
    fn a_missed_reminder_shows_until_the_end_only() {
        let item = event("a", "2026-11-06T10:00", "2026-11-06T10:30", Some(10));
        assert_eq!(
            ids(&with(vec![item.clone()]).take_due(at("2026-11-06T10:15"))),
            ["a"]
        );
        assert_eq!(
            with(vec![item]).take_due(at("2026-11-06T11:00")),
            Vec::<Item>::new()
        );
    }

    #[test]
    fn moving_an_item_shows_it_again() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T10:30",
            Some(10),
        )]);
        assert_eq!(store.take_due(at("2026-11-06T09:55")).len(), 1);
        store
            .put(event("a", "2026-11-06T15:00", "2026-11-06T15:30", Some(10)))
            .unwrap();
        assert_eq!(store.take_due(at("2026-11-06T10:00")), Vec::<Item>::new());
        assert_eq!(store.take_due(at("2026-11-06T14:50")).len(), 1);
    }

    #[test]
    fn later_brings_an_event_back_in_ten_minutes() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T11:00",
            Some(10),
        )]);
        store.take_due(at("2026-11-06T09:50"));
        store.snooze("a", at("2026-11-06T09:50"));
        assert_eq!(
            store.next_wake(at("2026-11-06T09:50")),
            Some(at("2026-11-06T10:00"))
        );
        assert_eq!(store.take_due(at("2026-11-06T09:59")), Vec::<Item>::new());
        assert_eq!(store.take_due(at("2026-11-06T10:00")).len(), 1);
        assert_eq!(store.take_due(at("2026-11-06T10:01")), Vec::<Item>::new());
    }

    #[test]
    fn later_brings_a_deadline_back_the_next_day() {
        let mut store = with(vec![deadline(
            "a",
            "2026-11-02T09:00",
            "2026-11-13T18:00",
            Some(4320),
        )]);
        store.take_due(at("2026-11-10T18:00"));
        store.snooze("a", at("2026-11-10T18:00"));
        assert_eq!(
            store.next_wake(at("2026-11-10T18:00")),
            Some(at("2026-11-11T18:00"))
        );
    }

    #[test]
    fn later_does_not_go_past_the_end() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T10:05",
            Some(0),
        )]);
        store.take_due(at("2026-11-06T10:00"));
        store.snooze("a", at("2026-11-06T10:00"));
        assert_eq!(store.next_wake(at("2026-11-06T10:00")), None);
    }

    #[test]
    fn waits_for_the_soonest() {
        let store = with(vec![
            event("a", "2026-11-08T10:00", "2026-11-08T11:00", Some(0)),
            event("b", "2026-11-07T10:00", "2026-11-07T11:00", Some(60)),
        ]);
        assert_eq!(
            store.next_wake(at("2026-11-06T10:00")),
            Some(at("2026-11-07T09:00"))
        );
    }

    #[test]
    fn removing_forgets_everything() {
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T11:00",
            Some(0),
        )]);
        store.remove("a");
        assert_eq!(store.items, Vec::<Item>::new());
        assert_eq!(store.next_wake(at("2026-11-06T09:00")), None);
    }

    #[test]
    fn survives_a_restart() {
        let folder = tempfile::tempdir().unwrap();
        let mut store = with(vec![event(
            "a",
            "2026-11-06T10:00",
            "2026-11-06T11:00",
            Some(10),
        )]);
        store.take_due(at("2026-11-06T09:50"));
        store.save(folder.path()).unwrap();
        let mut again = Store::load(folder.path());
        assert_eq!(again.items, store.items);
        assert_eq!(again.take_due(at("2026-11-06T09:51")), Vec::<Item>::new());
    }

    #[test]
    fn starts_empty_without_a_file_or_with_a_broken_one() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(Store::load(folder.path()).items, Vec::<Item>::new());
        fs::write(folder.path().join(FILE), "not json").unwrap();
        assert_eq!(Store::load(folder.path()).items, Vec::<Item>::new());
    }
}
