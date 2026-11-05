//! Turns a template into the activity Discord expects, leaving out what it would refuse.

use std::ops::RangeInclusive;

use serde_json::{Map, Value, json};

use crate::store::{Kind, Show, Template, Time};

const TEXT_LENGTHS: RangeInclusive<usize> = 2..=128;
const MAX_BUTTONS: usize = 2;
const BUTTON_LABEL_LENGTHS: RangeInclusive<usize> = 1..=32;
const MAX_URL_LENGTH: usize = 512;

fn text(value: &str) -> Option<String> {
    let value = value.trim();
    TEXT_LENGTHS
        .contains(&value.chars().count())
        .then(|| value.to_string())
}

pub fn is_web_address(value: &str) -> bool {
    value.len() <= MAX_URL_LENGTH && (value.starts_with("https://") || value.starts_with("http://"))
}

fn address(value: &str) -> Option<String> {
    let value = value.trim();
    is_web_address(value).then(|| value.to_string())
}

fn insert(object: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        object.insert(key.to_string(), Value::String(value));
    }
}

fn assets(template: &Template) -> Map<String, Value> {
    let mut assets = Map::new();
    let large = template.large.trim();
    let small = template.small.trim();
    if !large.is_empty() {
        assets.insert("large_image".to_string(), json!(large));
        insert(&mut assets, "large_text", text(&template.large_text));
        insert(&mut assets, "large_url", address(&template.large_url));
    }
    if !small.is_empty() {
        assets.insert("small_image".to_string(), json!(small));
        insert(&mut assets, "small_text", text(&template.small_text));
        insert(&mut assets, "small_url", address(&template.small_url));
    }
    assets
}

/// Rich Presence takes its times in seconds.
fn timestamps(template: &Template, since: i64) -> Option<Value> {
    let since = since / 1000;
    match template.time {
        Time::None => None,
        Time::Elapsed => Some(json!({ "start": since })),
        Time::Left => {
            let minutes = i64::from(template.minutes.filter(|minutes| *minutes > 0)?);
            Some(json!({ "end": since + minutes * 60 }))
        }
        Time::Dates => {
            let mut times = Map::new();
            if let Some(from) = template.from {
                times.insert("start".to_string(), json!(from / 1000));
            }
            if let Some(to) = template.to {
                times.insert("end".to_string(), json!(to / 1000));
            }
            (!times.is_empty()).then_some(Value::Object(times))
        }
    }
}

fn buttons(template: &Template) -> Vec<Value> {
    template
        .buttons
        .iter()
        .filter_map(|button| {
            let label = button.label.trim();
            let url = address(&button.url)?;
            BUTTON_LABEL_LENGTHS
                .contains(&label.chars().count())
                .then(|| json!({ "label": label, "url": url }))
        })
        .take(MAX_BUTTONS)
        .collect()
}

pub fn activity(template: &Template, since: i64) -> Value {
    let mut activity = Map::new();
    let kind = match template.kind {
        Kind::Playing => 0,
        Kind::Listening => 2,
        Kind::Watching => 3,
        Kind::Competing => 5,
    };
    let show = match template.show {
        Show::Name => 0,
        Show::State => 1,
        Show::Details => 2,
    };
    activity.insert("type".to_string(), json!(kind));
    activity.insert("status_display_type".to_string(), json!(show));
    insert(&mut activity, "name", text(&template.title));
    insert(&mut activity, "details", text(&template.details));
    insert(&mut activity, "details_url", address(&template.details_url));
    insert(&mut activity, "state", text(&template.state));
    insert(&mut activity, "state_url", address(&template.state_url));
    let assets = assets(template);
    if !assets.is_empty() {
        activity.insert("assets".to_string(), Value::Object(assets));
    }
    if let Some(times) = timestamps(template, since) {
        activity.insert("timestamps".to_string(), times);
    }
    if let (Some(size), Some(max)) = (template.size, template.max)
        && size > 0
        && size <= max
    {
        activity.insert(
            "party".to_string(),
            json!({ "id": template.id, "size": [size, max] }),
        );
    }
    let buttons = buttons(template);
    if !buttons.is_empty() {
        activity.insert("buttons".to_string(), Value::Array(buttons));
    }
    Value::Object(activity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Button;

    fn button(label: &str, url: &str) -> Button {
        Button {
            label: label.into(),
            url: url.into(),
        }
    }

    #[test]
    fn an_empty_template_says_only_its_type() {
        assert_eq!(
            activity(&Template::default(), 0),
            json!({ "type": 0, "status_display_type": 0 })
        );
    }

    #[test]
    fn gives_every_type_the_number_discord_knows() {
        let number = |kind| {
            activity(
                &Template {
                    kind,
                    ..Template::default()
                },
                0,
            )["type"]
                .clone()
        };
        assert_eq!(number(Kind::Playing), json!(0));
        assert_eq!(number(Kind::Listening), json!(2));
        assert_eq!(number(Kind::Watching), json!(3));
        assert_eq!(number(Kind::Competing), json!(5));
    }

    #[test]
    fn leaves_out_texts_discord_would_refuse() {
        let template = Template {
            details: " a ".into(),
            state: "x".repeat(129),
            title: "  Arthex  ".into(),
            ..Template::default()
        };
        let activity = activity(&template, 0);
        assert!(activity.get("details").is_none());
        assert!(activity.get("state").is_none());
        assert_eq!(activity["name"], json!("Arthex"));
    }

    #[test]
    fn takes_only_web_addresses() {
        let template = Template {
            details_url: "file:///c:/secret".into(),
            state_url: "https://example.com".into(),
            ..Template::default()
        };
        let activity = activity(&template, 0);
        assert!(activity.get("details_url").is_none());
        assert_eq!(activity["state_url"], json!("https://example.com"));
    }

    #[test]
    fn the_texts_of_an_image_need_the_image() {
        let without = Template {
            large_text: "Hover".into(),
            ..Template::default()
        };
        assert!(activity(&without, 0).get("assets").is_none());
        let with = Template {
            large: "logo".into(),
            large_text: "Hover".into(),
            small: "https://example.com/a.png".into(),
            ..Template::default()
        };
        assert_eq!(
            activity(&with, 0)["assets"],
            json!({ "large_image": "logo", "large_text": "Hover", "small_image": "https://example.com/a.png" })
        );
    }

    #[test]
    fn counts_time_from_the_activation_or_the_dates() {
        let with = |time, minutes, from, to| Template {
            time,
            minutes,
            from,
            to,
            ..Template::default()
        };
        let since = 1_000_000;
        assert_eq!(
            activity(&with(Time::Elapsed, None, None, None), since)["timestamps"],
            json!({ "start": 1000 })
        );
        assert_eq!(
            activity(&with(Time::Left, Some(2), None, None), since)["timestamps"],
            json!({ "end": 1120 })
        );
        assert_eq!(
            activity(&with(Time::Dates, None, Some(5000), Some(9000)), since)["timestamps"],
            json!({ "start": 5, "end": 9 })
        );
        assert!(
            activity(&with(Time::Left, Some(0), None, None), since)
                .get("timestamps")
                .is_none()
        );
        assert!(
            activity(&with(Time::None, Some(2), None, None), since)
                .get("timestamps")
                .is_none()
        );
    }

    #[test]
    fn a_party_needs_a_size_within_its_maximum() {
        let with = |size, max| Template {
            id: "t".into(),
            size,
            max,
            ..Template::default()
        };
        assert_eq!(
            activity(&with(Some(2), Some(4)), 0)["party"],
            json!({ "id": "t", "size": [2, 4] })
        );
        assert!(activity(&with(Some(5), Some(4)), 0).get("party").is_none());
        assert!(activity(&with(Some(2), None), 0).get("party").is_none());
    }

    #[test]
    fn keeps_two_valid_buttons_at_most() {
        let template = Template {
            buttons: vec![
                button("", "https://example.com/empty"),
                button("No address", "example.com"),
                button("First", "https://example.com/1"),
                button("Second", "https://example.com/2"),
                button("Third", "https://example.com/3"),
            ],
            ..Template::default()
        };
        assert_eq!(
            activity(&template, 0)["buttons"],
            json!([
                { "label": "First", "url": "https://example.com/1" },
                { "label": "Second", "url": "https://example.com/2" },
            ])
        );
    }

    #[test]
    fn names_what_the_status_shows() {
        let number = |show| {
            activity(
                &Template {
                    show,
                    ..Template::default()
                },
                0,
            )["status_display_type"]
                .clone()
        };
        assert_eq!(number(Show::Name), json!(0));
        assert_eq!(number(Show::State), json!(1));
        assert_eq!(number(Show::Details), json!(2));
    }
}
