use std::{fs, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};

const FILE: &str = "companion.json";
const MAX_NAME: usize = 20;
const XP_PER_LEVEL: u32 = 50;
pub const PET_GOAL: u32 = 30;
/// As many as a round can give.
const LONGEST_CATCH: u32 = 25;

#[derive(Deserialize)]
struct ShopGame {
    id: String,
    price: u32,
}

#[derive(Deserialize)]
struct ShopItem {
    id: String,
    price: u32,
}

/// What the shop sells and for how much, which the interface shows from the same file.
#[derive(Deserialize)]
struct Shop {
    games: Vec<ShopGame>,
    wear: Vec<ShopItem>,
}

fn shop() -> Option<&'static Shop> {
    static SHOP: OnceLock<Option<Shop>> = OnceLock::new();
    SHOP.get_or_init(|| serde_json::from_str(include_str!("../../shop.json")).ok())
        .as_ref()
}

fn game_price(game: &str) -> Option<u32> {
    let games = &shop()?.games;
    Some(games.iter().find(|entry| entry.id == game)?.price)
}

fn item_price(item: &str) -> Option<u32> {
    let wear = &shop()?.wear;
    Some(wear.iter().find(|entry| entry.id == item)?.price)
}

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
    pub fn reward(self) -> (u32, u32) {
        match self {
            Self::Catch { caught } => {
                let pay = 5 + caught.min(LONGEST_CATCH) * 3 / 5;
                (pay, pay)
            }
            Self::Memory => (30, 30),
            Self::HideAndSeek => (40, 40),
        }
    }

    const fn key(self) -> &'static str {
        match self {
            Self::Catch { .. } => "catch",
            Self::Memory => "memory",
            Self::HideAndSeek => "hide",
        }
    }
}

/// Finding the word pays the most of all, and more with few tries.
pub fn word_reward(tries: u32) -> (u32, u32) {
    (
        50 + 40u32.saturating_sub(tries * 2),
        50 + 30u32.saturating_sub(tries),
    )
}

/// The kibble that comes with leaving `level` for the next one.
const fn level_bonus(level: u32) -> u32 {
    5 + level.saturating_sub(1) * 5 / 2
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
    pub games: Vec<String>,
    pub pets: u32,
    #[serde(default = "default_on")]
    pub on_screen: bool,
    pub frequency: Frequency,
    pub reduced_motion: bool,
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
        self.games = Vec::new();
        self.pets = 0;
        self.wear = None;
        Ok(())
    }

    /// A reward grows with the level, and each level passed adds kibble of its own.
    fn outcome(&self, xp: u32, kibble: u32) -> (Gain, u32, u32) {
        let xp = xp.saturating_add(self.level);
        let mut kibble = kibble.saturating_add(self.level);
        let first = self.level.max(1);
        let mut level = first;
        let mut left = self.xp.saturating_add(xp);
        while left >= level * XP_PER_LEVEL {
            left -= level * XP_PER_LEVEL;
            kibble = kibble.saturating_add(level_bonus(level));
            level += 1;
        }
        let gain = Gain {
            xp,
            kibble,
            level_up: level > first,
        };
        (gain, level, left)
    }

    /// What a game would give now, for the screen that says so before it is granted.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn would_gain(&self, game: Game) -> Gain {
        let (xp, kibble) = game.reward();
        self.outcome(xp, kibble).0
    }

    pub fn grant(&mut self, xp: u32, kibble: u32) -> Gain {
        let (gain, level, left) = self.outcome(xp, kibble);
        self.level = level;
        self.xp = left;
        self.kibble = self.kibble.saturating_add(gain.kibble);
        gain
    }

    /// A game that costs nothing is everybody's from the start.
    pub fn has_game(&self, game: &str) -> bool {
        game_price(game) == Some(0) || self.games.iter().any(|owned| owned == game)
    }

    pub fn finish(&mut self, game: Game) -> Result<Gain, String> {
        if !self.has_game(game.key()) {
            return Err("game not unlocked".to_string());
        }
        let (xp, kibble) = game.reward();
        Ok(self.grant(xp, kibble))
    }

    /// A stroke of the "Pet" game, which pays each time it is full and starts over.
    pub fn pet(&mut self) -> Option<Gain> {
        self.pets += 1;
        if self.pets < PET_GOAL {
            return None;
        }
        self.pets = 0;
        Some(self.grant(10, 10))
    }

    pub fn buy_game(&mut self, game: &str) -> Result<(), String> {
        let price = game_price(game).ok_or("unknown game")?;
        if self.has_game(game) {
            return Err("already owned".to_string());
        }
        if self.kibble < price {
            return Err("not enough kibble".to_string());
        }
        self.kibble -= price;
        self.games.push(game.to_string());
        Ok(())
    }

    pub fn buy(&mut self, item: &str) -> Result<(), String> {
        let price = item_price(item).ok_or("unknown item")?;
        if self.owned.iter().any(|owned| owned == item) {
            return Err("already owned".to_string());
        }
        if self.kibble < price {
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
        let gain = progress.grant(54, 5);
        assert!(gain.level_up);
        assert_eq!((gain.xp, gain.kibble), (55, 11));
        assert_eq!((progress.level, progress.xp, progress.kibble), (2, 5, 11));
    }

    #[test]
    fn a_big_gain_can_pass_several_levels() {
        let mut progress = adopted();
        let gain = progress.grant(50 + 100 + 2, 0);
        assert_eq!((progress.level, progress.xp), (3, 3));
        assert_eq!(gain.kibble, 1 + 5 + 7);
    }

    #[test]
    fn rewards_grow_with_the_level() {
        let mut progress = adopted();
        progress.level = 4;
        assert_eq!(
            progress.grant(30, 30),
            Gain {
                xp: 34,
                kibble: 34,
                level_up: false
            }
        );
        assert_eq!(progress.would_gain(Game::HideAndSeek).xp, 44);
    }

    #[test]
    fn what_a_game_would_give_is_what_it_gives() {
        let mut progress = adopted();
        progress.games.push("hide".to_string());
        progress.xp = 40;
        let expected = progress.would_gain(Game::HideAndSeek);
        assert_eq!(progress.finish(Game::HideAndSeek), Ok(expected));
        assert!(expected.level_up);
    }

    #[test]
    fn catching_more_pays_more_within_a_limit() {
        assert_eq!(Game::Catch { caught: 0 }.reward(), (5, 5));
        assert_eq!(Game::Catch { caught: 10 }.reward(), (11, 11));
        assert_eq!(Game::Catch { caught: 9999 }.reward(), (20, 20));
    }

    #[test]
    fn finding_a_word_pays_the_most_and_more_with_few_tries() {
        let long_game = word_reward(100);
        assert!(long_game.0 > Game::HideAndSeek.reward().0);
        assert!(long_game.1 > Game::HideAndSeek.reward().1);
        assert!(word_reward(3).0 > word_reward(30).0);
        assert_eq!(long_game, (50, 50));
        assert_eq!(word_reward(0), (90, 80));
    }

    #[test]
    fn petting_pays_each_time_it_is_full() {
        let mut progress = adopted();
        for _ in 0..2 {
            for _ in 0..PET_GOAL - 1 {
                assert_eq!(progress.pet(), None);
            }
            let gain = progress.pet().unwrap();
            assert_eq!((gain.xp, gain.kibble), (11, 11));
            assert_eq!(progress.pets, 0);
        }
        assert_eq!(progress.kibble, 22);
    }

    #[test]
    fn the_shop_says_what_the_games_pay() {
        let games = &shop().unwrap().games;
        let shown = |id: &str| -> (u32, u32) {
            let text: serde_json::Value =
                serde_json::from_str(include_str!("../../shop.json")).unwrap();
            let games = text["games"].as_array().unwrap();
            let game = games.iter().find(|game| game["id"] == id).unwrap();
            (
                u32::try_from(game["xp"].as_u64().unwrap()).unwrap(),
                u32::try_from(game["kibble"].as_u64().unwrap()).unwrap(),
            )
        };
        assert_eq!(games.len(), 5);
        assert_eq!(shown("pet"), (10, 10));
        assert_eq!(
            shown("catch"),
            Game::Catch {
                caught: LONGEST_CATCH
            }
            .reward()
        );
        assert_eq!(shown("memory"), Game::Memory.reward());
        assert_eq!(shown("hide"), Game::HideAndSeek.reward());
        assert_eq!(shown("flair"), word_reward(u32::MAX / 4));
    }

    #[test]
    fn each_game_costs_and_pays_more_than_the_one_before() {
        let games: Vec<(u32, u32)> = [
            ("pet", (10, 10)),
            (
                "catch",
                Game::Catch {
                    caught: LONGEST_CATCH,
                }
                .reward(),
            ),
            ("memory", Game::Memory.reward()),
            ("hide", Game::HideAndSeek.reward()),
            ("flair", word_reward(u32::MAX / 4)),
        ]
        .iter()
        .map(|(id, (xp, _))| (game_price(id).unwrap(), *xp))
        .collect();
        assert!(
            games
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0 && pair[0].1 < pair[1].1)
        );
    }

    #[test]
    fn only_petting_is_open_at_first() {
        let mut progress = adopted();
        assert!(progress.has_game("pet"));
        for game in ["catch", "memory", "hide", "flair"] {
            assert!(!progress.has_game(game), "{game}");
        }
        assert!(progress.finish(Game::Memory).is_err());
        assert_eq!(progress.xp, 0);
    }

    #[test]
    fn a_game_is_bought_once_and_never_on_credit() {
        let mut progress = adopted();
        assert!(progress.buy_game("catch").is_err());
        progress.kibble = 100;
        progress.buy_game("catch").unwrap();
        assert_eq!(progress.kibble, 40);
        assert!(progress.has_game("catch"));
        assert!(progress.finish(Game::Catch { caught: 5 }).is_ok());
        assert!(progress.buy_game("catch").is_err());
        assert!(progress.buy_game("pet").is_err());
        assert!(progress.buy_game("chess").is_err());
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
        progress.kibble = 30;
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
        progress.kibble = 30;
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
