//! The documents and their categories: a Markdown file per document, and an index for the rest.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const INDEX: &str = "binder.json";
const DOCUMENTS: &str = "documents";
const LONGEST_NAME: usize = 40;
const LONGEST_TITLE: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub folded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    title: String,
    category: Option<String>,
    updated: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Index {
    categories: Vec<Category>,
    documents: Vec<Entry>,
}

/// A document as the interface sees it, newest first.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[serde(flatten)]
    entry: Entry,
    pub text: String,
}

/// An id becomes a file name: only what cannot leave the folder is accepted.
fn check_id(id: &str) -> Result<(), String> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err("invalid id".to_string())
    }
}

/// In one rename, so that a crash in the middle never loses anything.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}

pub struct Library {
    folder: PathBuf,
    index: Index,
}

impl Library {
    /// A broken index only loses the order and the categories: the text of the documents stays.
    pub fn load(folder: &Path) -> Self {
        let index = fs::read(folder.join(INDEX))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            folder: folder.to_path_buf(),
            index,
        }
    }

    fn document_path(&self, id: &str) -> PathBuf {
        self.folder.join(DOCUMENTS).join(format!("{id}.md"))
    }

    fn save_index(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&self.index).map_err(|error| error.to_string())?;
        write_atomic(&self.folder.join(INDEX), &bytes).map_err(|error| error.to_string())
    }

    pub fn categories(&self) -> &[Category] {
        &self.index.categories
    }

    pub fn documents(&self) -> Vec<Document> {
        let mut entries = self.index.documents.clone();
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.updated));
        entries
            .into_iter()
            .map(|entry| Document {
                text: fs::read_to_string(self.document_path(&entry.id)).unwrap_or_default(),
                entry,
            })
            .collect()
    }

    fn check_category(&self, category: Option<&str>) -> Result<(), String> {
        match category {
            Some(id) if !self.index.categories.iter().any(|known| known.id == id) => {
                Err("unknown category".to_string())
            }
            _ => Ok(()),
        }
    }

    /// Creates the document if it is new, and says when it was saved.
    pub fn save_document(
        &mut self,
        id: &str,
        title: &str,
        category: Option<&str>,
        text: &str,
        now: u64,
    ) -> Result<u64, String> {
        check_id(id)?;
        self.check_category(category)?;
        if title.chars().count() > LONGEST_TITLE {
            return Err("title too long".to_string());
        }
        let path = self.document_path(id);
        fs::create_dir_all(self.folder.join(DOCUMENTS)).map_err(|error| error.to_string())?;
        write_atomic(&path, text.as_bytes()).map_err(|error| error.to_string())?;
        let entry = Entry {
            id: id.to_string(),
            title: title.to_string(),
            category: category.map(str::to_string),
            updated: now,
        };
        match self.index.documents.iter_mut().find(|known| known.id == id) {
            Some(known) => *known = entry,
            None => self.index.documents.push(entry),
        }
        self.save_index()?;
        Ok(now)
    }

    pub fn remove_document(&mut self, id: &str) -> Result<(), String> {
        check_id(id)?;
        self.index.documents.retain(|known| known.id != id);
        self.save_index()?;
        match fs::remove_file(self.document_path(id)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.to_string()),
            _ => Ok(()),
        }
    }

    /// Moving a document does not count as changing it.
    pub fn move_document(&mut self, id: &str, category: Option<&str>) -> Result<(), String> {
        self.check_category(category)?;
        let known = self
            .index
            .documents
            .iter_mut()
            .find(|known| known.id == id)
            .ok_or("unknown document")?;
        known.category = category.map(str::to_string);
        self.save_index()
    }

    /// Creates the category if it is new, or renames it.
    pub fn save_category(&mut self, id: &str, name: &str) -> Result<(), String> {
        check_id(id)?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > LONGEST_NAME {
            return Err("invalid name".to_string());
        }
        match self
            .index
            .categories
            .iter_mut()
            .find(|known| known.id == id)
        {
            Some(known) => name.clone_into(&mut known.name),
            None => self.index.categories.push(Category {
                id: id.to_string(),
                name: name.to_string(),
                folded: false,
            }),
        }
        self.save_index()
    }

    /// Its documents stay, without a category.
    pub fn remove_category(&mut self, id: &str) -> Result<(), String> {
        self.index.categories.retain(|known| known.id != id);
        for document in &mut self.index.documents {
            if document.category.as_deref() == Some(id) {
                document.category = None;
            }
        }
        self.save_index()
    }

    pub fn fold_category(&mut self, id: &str, folded: bool) -> Result<(), String> {
        let known = self
            .index
            .categories
            .iter_mut()
            .find(|known| known.id == id)
            .ok_or("unknown category")?;
        known.folded = folded;
        self.save_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let folder = tempfile::tempdir().unwrap();
        let library = Library::load(folder.path());
        (folder, library)
    }

    #[test]
    fn a_document_survives_a_restart_with_its_category() {
        let (folder, mut library) = library();
        library.save_category("maths", "Maths").unwrap();
        library
            .save_document("a", "Matrices", Some("maths"), "# Produit", 5)
            .unwrap();
        let again = Library::load(folder.path());
        let documents = again.documents();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].text, "# Produit");
        assert_eq!(documents[0].entry.category.as_deref(), Some("maths"));
        assert_eq!(again.categories()[0].name, "Maths");
    }

    #[test]
    fn saving_again_replaces_it_and_the_newest_comes_first() {
        let (_folder, mut library) = library();
        library.save_document("a", "A", None, "un", 1).unwrap();
        library.save_document("b", "B", None, "deux", 2).unwrap();
        assert_eq!(library.documents()[0].entry.id, "b");
        library.save_document("a", "A2", None, "trois", 3).unwrap();
        let documents = library.documents();
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[0].entry.title, "A2");
        assert_eq!(documents[0].text, "trois");
    }

    #[test]
    fn an_id_never_leaves_the_folder() {
        let (_folder, mut library) = library();
        for id in ["../evil", "a/b", "a\\b", "", "a.b", &"x".repeat(65)] {
            assert!(library.save_document(id, "", None, "", 1).is_err(), "{id}");
            assert!(library.remove_document(id).is_err(), "{id}");
            assert!(library.save_category(id, "Nom").is_err(), "{id}");
        }
    }

    #[test]
    fn a_document_only_goes_in_a_category_that_exists() {
        let (_folder, mut library) = library();
        assert!(library.save_document("a", "", Some("non"), "", 1).is_err());
        library.save_document("a", "", None, "", 1).unwrap();
        assert!(library.move_document("a", Some("non")).is_err());
        assert!(library.move_document("inconnu", None).is_err());
    }

    #[test]
    fn moving_a_document_keeps_its_date_and_text() {
        let (_folder, mut library) = library();
        library.save_category("c", "Cours").unwrap();
        library.save_document("a", "A", None, "texte", 7).unwrap();
        library.move_document("a", Some("c")).unwrap();
        let document = &library.documents()[0];
        assert_eq!(document.entry.category.as_deref(), Some("c"));
        assert_eq!(
            (document.entry.updated, document.text.as_str()),
            (7, "texte")
        );
    }

    #[test]
    fn removing_a_category_keeps_its_documents() {
        let (_folder, mut library) = library();
        library.save_category("c", "Cours").unwrap();
        library
            .save_document("a", "A", Some("c"), "texte", 1)
            .unwrap();
        library.remove_category("c").unwrap();
        assert_eq!(library.categories().len(), 0);
        let document = &library.documents()[0];
        assert_eq!(document.entry.category, None);
        assert_eq!(document.text, "texte");
    }

    #[test]
    fn a_category_needs_a_short_name_and_can_be_renamed() {
        let (_folder, mut library) = library();
        assert!(library.save_category("c", "   ").is_err());
        assert!(library.save_category("c", &"x".repeat(41)).is_err());
        library.save_category("c", " Cours ").unwrap();
        library.save_category("c", "Révisions").unwrap();
        assert_eq!(library.categories().len(), 1);
        assert_eq!(library.categories()[0].name, "Révisions");
    }

    #[test]
    fn a_category_stays_folded() {
        let (folder, mut library) = library();
        library.save_category("c", "Cours").unwrap();
        library.fold_category("c", true).unwrap();
        assert!(Library::load(folder.path()).categories()[0].folded);
        assert!(library.fold_category("autre", true).is_err());
    }

    #[test]
    fn a_removed_document_is_gone_and_removing_twice_is_fine() {
        let (folder, mut library) = library();
        library.save_document("a", "A", None, "texte", 1).unwrap();
        library.remove_document("a").unwrap();
        library.remove_document("a").unwrap();
        assert_eq!(library.documents().len(), 0);
        assert!(!folder.path().join(DOCUMENTS).join("a.md").exists());
    }

    #[test]
    fn a_broken_index_starts_empty() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join(INDEX), "pas du json").unwrap();
        assert_eq!(Library::load(folder.path()).documents().len(), 0);
    }
}
