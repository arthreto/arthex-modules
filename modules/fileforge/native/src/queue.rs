//! The list of files and its settings, without any of the work.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::format::{self, Kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Convert,
    Compress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ready,
    Waiting,
    Working,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// From 10 to 100.
    pub quality: u32,
    /// The height of a video, `None` to keep it.
    pub height: Option<u32>,
    pub strip_metadata: bool,
    /// How strong a compression is, from 10 to 100, as a quality: lower is smaller.
    pub level: u32,
    /// Next to the original when `None`.
    pub folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: 80,
            height: None,
            strip_metadata: false,
            level: 60,
            folder: None,
        }
    }
}

impl Settings {
    pub fn normalized(mut self) -> Self {
        self.quality = self.quality.clamp(10, 100);
        self.level = self.level.clamp(10, 100);
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    /// What it becomes when it is converted.
    pub target: String,
    /// How it runs, once started.
    pub mode: Mode,
    /// The batch it was started with: the total only counts the latest one.
    pub run: u64,
    pub status: Status,
    pub progress: u8,
    pub output: Option<String>,
    pub output_size: Option<u64>,
    /// A compression that would not have saved anything: the file stays as it is.
    pub no_gain: bool,
    /// `needsOffice` or `failed`.
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub items: Vec<Item>,
    pub mode: Mode,
    pub settings: Settings,
    #[serde(skip)]
    next_id: u64,
    #[serde(skip)]
    run: u64,
}

impl Queue {
    pub fn new(settings: Settings) -> Self {
        Self {
            items: Vec::new(),
            mode: Mode::Convert,
            settings,
            next_id: 0,
            run: 0,
        }
    }

    /// Adds a file once. Anything else than a file of a known type is left out.
    pub fn add(&mut self, path: &Path, size: u64) -> Option<u64> {
        let text = path.display().to_string();
        let name = path.file_name()?.to_string_lossy().to_string();
        let extension = path.extension()?.to_string_lossy().to_string();
        let kind = format::kind_of(&extension)?;
        if let Some(known) = self
            .items
            .iter()
            .find(|item| item.path == text && item.status != Status::Done)
        {
            return Some(known.id);
        }
        self.next_id += 1;
        self.items.push(Item {
            id: self.next_id,
            path: text,
            name,
            kind,
            size,
            target: format::default_target(kind, &extension).to_string(),
            mode: self.mode,
            run: 0,
            status: Status::Ready,
            progress: 0,
            output: None,
            output_size: None,
            no_gain: false,
            error: None,
        });
        Some(self.next_id)
    }

    pub fn set_target(&mut self, id: u64, target: &str) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id)
            && format::is_allowed(item.kind, target)
        {
            item.target = target.to_string();
        }
    }

    /// For "Apply to all the images". A format a file cannot take leaves it as it is.
    pub fn set_kind_target(&mut self, kind: Kind, target: &str) {
        for item in &mut self.items {
            if item.kind == kind && item.status == Status::Ready && format::is_allowed(kind, target)
            {
                item.target = target.to_string();
            }
        }
    }

    /// Everything that can start, in the current mode.
    pub fn start(&mut self) {
        self.begin_run_if_idle();
        let (mode, run) = (self.mode, self.run);
        for item in &mut self.items {
            if item.status == Status::Ready {
                item.mode = mode;
                item.run = run;
                item.status = Status::Waiting;
            }
        }
    }

    /// What arrives during a run joins it; after one, it starts a new one.
    fn begin_run_if_idle(&mut self) {
        if !self.is_running() {
            self.run += 1;
        }
    }

    /// Queues one file for a right-click action.
    pub fn queue_now(&mut self, id: u64, mode: Mode, target: Option<&str>) {
        self.mode = mode;
        if let Some(target) = target {
            self.set_target(id, target);
        }
        self.begin_run_if_idle();
        let run = self.run;
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id)
            && item.status == Status::Ready
        {
            item.mode = mode;
            item.run = run;
            item.status = Status::Waiting;
        }
    }

    /// The next file to work on, marked as such.
    pub fn take_next(&mut self) -> Option<Item> {
        let item = self
            .items
            .iter_mut()
            .find(|item| item.status == Status::Waiting)?;
        item.status = Status::Working;
        item.progress = 0;
        Some(item.clone())
    }

    pub fn is_running(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item.status, Status::Waiting | Status::Working))
    }

    /// Stops everything: what had not finished is ready again.
    pub fn cancel(&mut self) {
        for item in &mut self.items {
            if matches!(item.status, Status::Waiting | Status::Working) {
                item.status = Status::Ready;
                item.progress = 0;
            }
        }
    }

    pub fn remove(&mut self, id: u64) {
        self.items
            .retain(|item| item.id != id || item.status == Status::Working);
    }

    /// Leaves only the work in progress.
    pub fn clear(&mut self) {
        self.items
            .retain(|item| matches!(item.status, Status::Waiting | Status::Working));
    }

    pub fn item_mut(&mut self, id: u64) -> Option<&mut Item> {
        self.items.iter_mut().find(|item| item.id == id)
    }
}

/// Where a result goes: never over an existing file.
pub fn free_output(
    folder: &Path,
    stem: &str,
    extension: &str,
    exists: &dyn Fn(&Path) -> bool,
) -> std::path::PathBuf {
    let first = folder.join(format!("{stem}.{extension}"));
    if !exists(&first) {
        return first;
    }
    (1..=u32::MAX)
        .map(|number| folder.join(format!("{stem} ({number}).{extension}")))
        .find(|path| !exists(path))
        .unwrap_or(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> Queue {
        Queue::new(Settings::default())
    }

    #[test]
    fn adds_a_file_once_and_only_known_types() {
        let mut queue = queue();
        let first = queue.add(Path::new("dir/a.png"), 10);
        assert_eq!(queue.add(Path::new("dir/a.png"), 10), first);
        assert_eq!(queue.add(Path::new("dir/a.exe"), 10), None);
        assert_eq!(queue.add(Path::new("dir/noextension"), 10), None);
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].name, "a.png");
        assert_eq!(queue.items[0].target, "jpeg");
    }

    #[test]
    fn refuses_a_format_the_file_cannot_take() {
        let mut queue = queue();
        let id = queue.add(Path::new("a.wav"), 1).unwrap();
        queue.set_target(id, "mp4");
        assert_eq!(queue.items[0].target, "mp3");
        queue.set_target(id, "flac");
        assert_eq!(queue.items[0].target, "flac");
    }

    #[test]
    fn applies_a_format_to_every_file_of_a_type() {
        let mut queue = queue();
        queue.add(Path::new("a.png"), 1);
        queue.add(Path::new("b.bmp"), 1);
        queue.add(Path::new("c.mp4"), 1);
        queue.set_kind_target(Kind::Image, "webp");
        assert_eq!(queue.items[0].target, "webp");
        assert_eq!(queue.items[1].target, "webp");
        assert_ne!(queue.items[2].target, "webp");
    }

    #[test]
    fn works_in_order_and_cancels_what_is_left() {
        let mut queue = queue();
        queue.add(Path::new("a.png"), 1);
        queue.add(Path::new("b.png"), 1);
        queue.start();
        assert!(queue.is_running());
        assert_eq!(queue.take_next().unwrap().name, "a.png");
        queue.item_mut(1).unwrap().status = Status::Done;
        assert_eq!(queue.take_next().unwrap().name, "b.png");
        assert!(queue.take_next().is_none());
        queue.cancel();
        assert_eq!(queue.items[0].status, Status::Done);
        assert_eq!(queue.items[1].status, Status::Ready);
        assert!(!queue.is_running());
    }

    #[test]
    fn files_started_after_a_run_form_a_new_one() {
        let mut queue = queue();
        let first = queue.add(Path::new("a.png"), 1).unwrap();
        queue.start();
        queue.take_next();
        queue.item_mut(first).unwrap().status = Status::Done;
        let second = queue.add(Path::new("b.png"), 1).unwrap();
        queue.start();
        let joining = queue.add(Path::new("c.png"), 1).unwrap();
        queue.queue_now(joining, Mode::Convert, None);
        let run_of = |queue: &Queue, id| queue.items.iter().find(|item| item.id == id).unwrap().run;
        assert_ne!(run_of(&queue, first), run_of(&queue, second));
        assert_eq!(run_of(&queue, second), run_of(&queue, joining));
    }

    #[test]
    fn a_right_click_starts_only_its_file() {
        let mut queue = queue();
        queue.add(Path::new("a.png"), 1);
        let id = queue.add(Path::new("b.png"), 1).unwrap();
        queue.queue_now(id, Mode::Compress, None);
        assert_eq!(queue.items[0].status, Status::Ready);
        assert_eq!(queue.items[1].status, Status::Waiting);
        assert_eq!(queue.items[1].mode, Mode::Compress);
        assert_eq!(queue.mode, Mode::Compress);
    }

    #[test]
    fn keeps_the_file_being_worked_on() {
        let mut queue = queue();
        queue.add(Path::new("a.png"), 1);
        queue.add(Path::new("b.png"), 1);
        queue.start();
        queue.take_next();
        queue.remove(1);
        queue.remove(2);
        assert_eq!(queue.items.len(), 1);
        queue.clear();
        assert_eq!(queue.items.len(), 1);
    }

    #[test]
    fn never_writes_over_a_file() {
        let taken = |path: &Path| {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            name == "a.jpg" || name == "a (1).jpg"
        };
        let found = free_output(Path::new("dir"), "a", "jpg", &taken);
        assert_eq!(found.file_name().unwrap().to_string_lossy(), "a (2).jpg");
        let free = free_output(Path::new("dir"), "b", "jpg", &taken);
        assert_eq!(free.file_name().unwrap().to_string_lossy(), "b.jpg");
    }

    #[test]
    fn settings_stay_in_their_range() {
        let settings = Settings {
            quality: 500,
            level: 0,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(settings.quality, 100);
        assert_eq!(settings.level, 10);
    }
}
