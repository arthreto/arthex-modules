use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

const FILE: &str = "companion.json";
const MAX_NAME: usize = 20;
const XP_PER_LEVEL: u32 = 25;
pub const PET_GOAL: u32 = 50;
const LONGEST_CATCH: u32 = 60;
/// The accessories of the shop and what they cost, in kibble.
pub const SHOP: [(&str, u32); 4] = [("scarf", 0), ("glasses", 150), ("bow", 200), ("crown", 500)];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Species {
    Fox,
    Dog,
    Cat,
    Wolf,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Species {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Fox => "fox",
            Self::Dog => "dog",
            Self::Cat => "cat",
            Self::Wolf => "wolf",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Frequency {
    Rare,
    #[default]
    Normal,
    Often,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl Frequency {
    /// The usual wait between two visits on the screen.
    pub const fn minutes(self) -> u32 {
        match self {
            Self::Rare => 120,
            Self::Normal => 45,
            Self::Often => 15,
        }
    }
}

/// What a finished game is worth is decided here, from what happened in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "game", rename_all = "camelCase")]
pub enum Game {
    Catch { caught: u32 },
    Memory,
    HideAndSeek,
}

impl Game {
    fn reward(self) -> (u32, u32) {
        match self {
            Self::Catch { caught } => {
                let caught = caught.min(LONGEST_CATCH);
                (10 + caught * 2, 5 + caught)
            }
            Self::Memory => (30, 20),
            Self::HideAndSeek => (30, 0),
        }
    }
}

/// Finding the word pays the most of all, and more with few tries.
pub fn word_reward(tries: u32) -> (u32, u32) {
    (
        40 + 40u32.saturating_sub(tries * 2),
        30 + 30u32.saturating_sub(tries),
    )
}

const fn default_on() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Progress {
    pub species: Option<Species>,
    pub name: String,
    pub level: u32,
    pub xp: u32,
    pub kibble: u32,
    pub wear: Option<String>,
    pub owned: Vec<String>,
    #[serde(default = "default_on")]
    pub on_screen: bool,
    pub frequency: Frequency,
    pub reduced_motion: bool,
    day: i64,
    pets: u32,
    pets_paid: bool,
}

/// What a finished game gave, for the screen that says so.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Gain {
    pub xp: u32,
    pub kibble: u32,
    pub level_up: bool,
}

impl Progress {
    pub fn load(folder: &Path) -> Self {
        fs::read(folder.join(FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Self {
                on_screen: true,
                ..Self::default()
            })
    }

    /// In one rename, so that a crash in the middle never loses a mascot.
    pub fn save(&self, folder: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        let temporary = folder.join(format!("{FILE}.tmp"));
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, folder.join(FILE)).map_err(|error| error.to_string())
    }

    pub const fn xp_needed(&self) -> u32 {
        self.level * XP_PER_LEVEL
    }

    pub fn adopt(&mut self, species: Species, name: &str) -> Result<(), String> {
        let name = name.trim();
        if self.species.is_some() {
            return Err("already adopted".to_string());
        }
        if name.is_empty() || name.chars().count() > MAX_NAME {
            return Err("invalid name".to_string());
        }
        self.species = Some(species);
        self.name = name.to_string();
        self.level = 1;
        self.xp = 0;
        self.kibble = 0;
        self.owned = Vec::new();
        self.wear = None;
        Ok(())
    }

    pub fn grant(&mut self, xp: u32, kibble: u32) -> Gain {
        self.kibble = self.kibble.saturating_add(kibble);
        self.xp = self.xp.saturating_add(xp);
        let mut level_up = false;
        while self.xp >= self.xp_needed() {
            self.xp -= self.xp_needed();
            self.level += 1;
            level_up = true;
        }
        Gain {
            xp,
            kibble,
            level_up,
        }
    }

    pub fn finish(&mut self, game: Game) -> Gain {
        let (xp, kibble) = game.reward();
        self.grant(xp, kibble)
    }

    fn roll_day(&mut self, day: i64) {
        if self.day != day {
            self.day = day;
            self.pets = 0;
            self.pets_paid = false;
        }
    }

    pub fn pets_today(&mut self, day: i64) -> u32 {
        self.roll_day(day);
        self.pets
    }

    /// A stroke of the "Pet" game, which pays once a day when it is full.
    pub fn pet(&mut self, day: i64) -> Option<Gain> {
        self.roll_day(day);
        if self.pets >= PET_GOAL {
            return None;
        }
        self.pets += 1;
        if self.pets == PET_GOAL && !self.pets_paid {
            self.pets_paid = true;
            return Some(self.grant(25, 10));
        }
        None
    }

    /// A click on the mascot.
    pub fn tap(&mut self) -> Gain {
        self.grant(1, 0)
    }

    pub fn buy(&mut self, item: &str) -> Result<(), String> {
        let (_, price) = SHOP
            .iter()
            .find(|(name, _)| *name == item)
            .ok_or("unknown item")?;
        if self.owned.iter().any(|owned| owned == item) {
            return Err("already owned".to_string());
        }
        if self.kibble < *price {
            return Err("not enough kibble".to_string());
        }
        self.kibble -= price;
        self.owned.push(item.to_string());
        self.wear = Some(item.to_string());
        Ok(())
    }

    pub fn put_on(&mut self, item: Option<&str>) -> Result<(), String> {
        if let Some(item) = item
            && !self.owned.iter().any(|owned| owned == item)
        {
            return Err("not owned".to_string());
        }
        self.wear = item.map(str::to_string);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adopted() -> Progress {
        let mut progress = Progress::default();
        progress.adopt(Species::Fox, " Kit ").unwrap();
        progress
    }

    #[test]
    fn an_adoption_starts_at_the_first_level() {
        let progress = adopted();
        assert_eq!(progress.name, "Kit");
        assert_eq!((progress.level, progress.xp, progress.kibble), (1, 0, 0));
        assert_eq!(progress.owned, Vec::<String>::new());
    }

    #[test]
    fn a_name_is_needed_and_stays_short() {
        let mut progress = Progress::default();
        assert!(progress.adopt(Species::Cat, "  ").is_err());
        assert!(progress.adopt(Species::Cat, &"x".repeat(21)).is_err());
        assert!(progress.species.is_none());
    }

    #[test]
    fn a_mascot_is_adopted_once() {
        let mut progress = adopted();
        assert!(progress.adopt(Species::Wolf, "Other").is_err());
        assert_eq!(progress.species, Some(Species::Fox));
    }

    #[test]
    fn experience_carries_over_to_the_next_level() {
        let mut progress = adopted();
        let gain = progress.grant(30, 5);
        assert!(gain.level_up);
        assert_eq!((progress.level, progress.xp, progress.kibble), (2, 5, 5));
    }

    #[test]
    fn a_big_gain_can_pass_several_levels() {
        let mut progress = adopted();
        progress.grant(25 + 50 + 3, 0);
        assert_eq!((progress.level, progress.xp), (3, 3));
    }

    #[test]
    fn catching_more_pays_more_within_a_limit() {
        assert_eq!(Game::Catch { caught: 0 }.reward(), (10, 5));
        assert_eq!(Game::Catch { caught: 10 }.reward(), (30, 15));
        assert_eq!(
            Game::Catch { caught: 9999 }.reward(),
            Game::Catch { caught: 60 }.reward()
        );
    }

    #[test]
    fn finding_a_word_pays_the_most_and_more_with_few_tries() {
        let long_game = word_reward(100);
        assert!(long_game.0 > Game::Memory.reward().0);
        assert!(long_game.1 > Game::Memory.reward().1);
        assert!(word_reward(3).0 > word_reward(30).0);
        assert_eq!(long_game, (40, 30));
    }

    #[test]
    fn petting_pays_once_when_full_and_comes_back_the_next_day() {
        let mut progress = adopted();
        for _ in 0..PET_GOAL - 1 {
            assert_eq!(progress.pet(1), None);
        }
        assert_eq!(progress.pet(1).map(|gain| gain.xp), Some(25));
        assert_eq!(progress.pet(1), None);
        assert_eq!(progress.pets_today(1), PET_GOAL);
        assert_eq!(progress.pets_today(2), 0);
    }

    #[test]
    fn a_click_can_end_a_level() {
        let mut progress = adopted();
        progress.grant(24, 0);
        assert!(progress.tap().level_up);
        assert_eq!((progress.level, progress.xp), (2, 0));
    }

    #[test]
    fn buying_costs_kibble_and_puts_the_item_on() {
        let mut progress = adopted();
        progress.kibble = 200;
        progress.buy("glasses").unwrap();
        assert_eq!(progress.kibble, 50);
        assert_eq!(progress.wear.as_deref(), Some("glasses"));
    }

    #[test]
    fn nothing_is_bought_twice_on_credit_or_out_of_the_shop() {
        let mut progress = adopted();
        assert!(progress.buy("crown").is_err());
        progress.buy("scarf").unwrap();
        assert!(progress.buy("scarf").is_err());
        progress.kibble = 9999;
        assert!(progress.buy("sword").is_err());
        assert_eq!(progress.kibble, 9999);
    }

    #[test]
    fn only_what_is_owned_can_be_worn() {
        let mut progress = adopted();
        assert!(progress.put_on(Some("crown")).is_err());
        progress.buy("scarf").unwrap();
        progress.put_on(Some("scarf")).unwrap();
        assert_eq!(progress.wear.as_deref(), Some("scarf"));
        progress.put_on(None).unwrap();
        assert_eq!(progress.wear, None);
    }

    #[test]
    fn survives_a_restart() {
        let folder = tempfile::tempdir().unwrap();
        let mut progress = adopted();
        progress.grant(5, 7);
        progress.save(folder.path()).unwrap();
        assert_eq!(Progress::load(folder.path()), progress);
    }

    #[test]
    fn starts_on_the_screen_without_a_file_or_with_a_broken_one() {
        let folder = tempfile::tempdir().unwrap();
        assert!(Progress::load(folder.path()).on_screen);
        fs::write(folder.path().join(FILE), "not json").unwrap();
        let progress = Progress::load(folder.path());
        assert!(progress.on_screen);
        assert_eq!(progress.species, None);
    }
}
