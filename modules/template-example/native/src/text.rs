//! The contents of text and code files, and the fictitious people that fill tables. Everything is
//! made from fixed lists, so the same choice always gives the same file.

use std::fmt::Write;

/// The language of what is written in the files.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    French,
    English,
}

impl Language {
    pub fn of(locale: &str) -> Self {
        if locale.starts_with("fr") {
            Self::French
        } else {
            Self::English
        }
    }

    pub fn pick<'a>(self, french: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::French => french,
            Self::English => english,
        }
    }
}

const FIRST_FR: [&str; 16] = [
    "Camille", "Lucas", "Léa", "Hugo", "Chloé", "Louis", "Manon", "Gabriel", "Inès", "Jules",
    "Emma", "Arthur", "Jade", "Nathan", "Zoé", "Raphaël",
];
const LAST_FR: [&str; 16] = [
    "Martin", "Bernard", "Thomas", "Petit", "Robert", "Richard", "Durand", "Dubois", "Moreau",
    "Laurent", "Simon", "Michel", "Lefebvre", "Leroy", "Roux", "David",
];
const CITIES_FR: [&str; 10] = [
    "Lyon",
    "Nantes",
    "Lille",
    "Bordeaux",
    "Toulouse",
    "Marseille",
    "Rennes",
    "Strasbourg",
    "Nice",
    "Paris",
];
const FIRST_EN: [&str; 16] = [
    "Olivia", "Liam", "Emma", "Noah", "Ava", "Ethan", "Sophia", "Mason", "Isabella", "James",
    "Mia", "Lucas", "Amelia", "Henry", "Harper", "Jack",
];
const LAST_EN: [&str; 16] = [
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Wilson",
    "Taylor", "Clark", "Lewis", "Walker", "Hall", "Young", "King",
];
const CITIES_EN: [&str; 10] = [
    "Boston", "Denver", "Seattle", "Austin", "Chicago", "Portland", "Atlanta", "Phoenix", "Dallas",
    "Miami",
];

/// One fictitious person of a table: the same `index` always gives the same person.
pub struct Person {
    pub id: usize,
    pub first: &'static str,
    pub last: &'static str,
    pub email: String,
    pub city: &'static str,
    /// Day of 2026, from 0.
    pub day: u32,
    /// In cents.
    pub cents: u64,
}

fn ascii_lowercase(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' | 'É' => 'e',
            'à' | 'â' => 'a',
            'î' | 'ï' => 'i',
            'ô' => 'o',
            'ù' | 'û' => 'u',
            'ç' => 'c',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

impl Person {
    pub fn new(index: usize, language: Language) -> Self {
        let (first, last, cities) = match language {
            Language::French => (FIRST_FR, LAST_FR, CITIES_FR),
            Language::English => (FIRST_EN, LAST_EN, CITIES_EN),
        };
        let first = first[index % 16];
        let last = last[(index * 7 + 3) % 16];
        Self {
            id: index + 1,
            first,
            last,
            email: format!(
                "{}.{}{}@example.com",
                ascii_lowercase(first),
                ascii_lowercase(last),
                if index >= 16 {
                    (index / 16).to_string()
                } else {
                    String::new()
                }
            ),
            city: cities[(index * 3) % 10],
            day: u32::try_from((index * 37) % 365).unwrap_or(0),
            cents: 500 + u64::try_from((index * 7919) % 50_000).unwrap_or(0),
        }
    }

    /// `2026-03-14`.
    pub fn date(&self) -> String {
        date_of_2026(self.day)
    }

    /// `128.50`.
    pub fn amount(&self) -> String {
        format!("{}.{:02}", self.cents / 100, self.cents % 100)
    }
}

const MONTHS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// The date of a day of 2026, counted from 0 for 1 January.
pub fn date_of_2026(mut day: u32) -> String {
    let mut month = 0;
    while month < 11 && day >= MONTHS[month] {
        day -= MONTHS[month];
        month += 1;
    }
    format!("2026-{:02}-{:02}", month + 1, day + 1)
}

/// The formats whose number of lines the user chooses.
pub const WITH_ROWS: [&str; 4] = ["csv", "json", "xml", "log"];

/// What a text or code file holds, or nothing for another format.
pub fn content(format: &str, rows: usize, language: Language) -> Option<String> {
    let french = language == Language::French;
    let people = || (0..rows).map(|index| Person::new(index, language));
    let text = match format {
        "csv" => {
            let mut text = language
                .pick(
                    "id,prenom,nom,email,ville,inscription,montant\n",
                    "id,first_name,last_name,email,city,signed_up,amount\n",
                )
                .to_string();
            for p in people() {
                let _ = writeln!(text, "{},{},{},{},{},{},{}", p.id, p.first, p.last, p.email, p.city, p.date(), p.amount());
            }
            text
        }
        "json" => {
            let keys = if french {
                ["prenom", "nom", "ville", "inscription", "montant", "actif"]
            } else {
                ["first_name", "last_name", "city", "signed_up", "amount", "active"]
            };
            let items: Vec<serde_json::Value> = people()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id, keys[0]: p.first, keys[1]: p.last, "email": p.email,
                        keys[2]: p.city, keys[3]: p.date(),
                        keys[4]: serde_json::Number::from_f64(f64::from(u32::try_from(p.cents).unwrap_or(0)) / 100.0),
                        keys[5]: p.id % 3 != 0,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&items).unwrap_or_default() + "\n"
        }
        "xml" => {
            let (root, item, first, last, city) = if french {
                ("personnes", "personne", "prenom", "nom", "ville")
            } else {
                ("people", "person", "first_name", "last_name", "city")
            };
            let mut text = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{root}>\n");
            for p in people() {
                let _ = write!(
                    text,
                    "  <{item} id=\"{}\">\n    <{first}>{}</{first}>\n    <{last}>{}</{last}>\n    <email>{}</email>\n    <{city}>{}</{city}>\n  </{item}>\n",
                    p.id, p.first, p.last, p.email, p.city
                );
            }
            text + &format!("</{root}>\n")
        }
        "log" => log(rows, language),
        "txt" => language
            .pick(
                "Fichier texte d’exemple créé par Template Example.\n\nLigne 1 : Le vif renard brun saute par-dessus le chien paresseux.\nLigne 2 : Portez ce vieux whisky au juge blond qui fume.\nLigne 3 : 0123456789 éàèùç ÉÀÈÙÇ œ € « » …\nLigne 4 : Tabulation\tici, puis une ligne vide.\n\nFin du fichier.\n",
                "Sample text file created by Template Example.\n\nLine 1: The quick brown fox jumps over the lazy dog.\nLine 2: Pack my box with five dozen liquor jugs.\nLine 3: 0123456789 éàèùç ÉÀÈÙÇ œ € « » …\nLine 4: A tab\there, then an empty line.\n\nEnd of file.\n",
            )
            .to_string(),
        "md" => language
            .pick(
                "# Document d’exemple\n\nUn paragraphe avec du **gras**, de l’*italique* et du `code`.\n\n## Une liste\n\n- Premier élément\n- Deuxième élément\n  - Sous-élément\n\n1. Un\n2. Deux\n\n## Un tableau\n\n| Colonne | Valeur |\n| ------- | -----: |\n| Alpha   |     12 |\n| Bêta    |     34 |\n\n> Une citation.\n\n```js\nconsole.log('Bonjour');\n```\n\n[Un lien](https://example.com)\n",
                "# Sample document\n\nA paragraph with **bold**, *italic* and `code`.\n\n## A list\n\n- First item\n- Second item\n  - Sub-item\n\n1. One\n2. Two\n\n## A table\n\n| Column | Value |\n| ------ | ----: |\n| Alpha  |    12 |\n| Beta   |    34 |\n\n> A quote.\n\n```js\nconsole.log('Hello');\n```\n\n[A link](https://example.com)\n",
            )
            .to_string(),
        "yaml" => language
            .pick(
                "application:\n  nom: exemple\n  version: 1.0.0\n  port: 8080\n  debug: false\nbase:\n  hote: localhost\n  port: 5432\n  utilisateurs:\n    - camille\n    - lucas\n",
                "application:\n  name: sample\n  version: 1.0.0\n  port: 8080\n  debug: false\ndatabase:\n  host: localhost\n  port: 5432\n  users:\n    - olivia\n    - liam\n",
            )
            .to_string(),
        "html" => language
            .pick(
                "<!doctype html>\n<html lang=\"fr\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>Page d’exemple</title>\n</head>\n<body>\n  <h1>Page d’exemple</h1>\n  <p>Un paragraphe et <a href=\"https://example.com\">un lien</a>.</p>\n  <ul>\n    <li>Premier élément</li>\n    <li>Deuxième élément</li>\n  </ul>\n  <form>\n    <label>Nom <input name=\"nom\"></label>\n    <button>Envoyer</button>\n  </form>\n</body>\n</html>\n",
                "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>Sample page</title>\n</head>\n<body>\n  <h1>Sample page</h1>\n  <p>A paragraph and <a href=\"https://example.com\">a link</a>.</p>\n  <ul>\n    <li>First item</li>\n    <li>Second item</li>\n  </ul>\n  <form>\n    <label>Name <input name=\"name\"></label>\n    <button>Send</button>\n  </form>\n</body>\n</html>\n",
            )
            .to_string(),
        "css" => ":root {\n  --accent: #4cc2ff;\n  --text: #1f2328;\n}\n\nbody {\n  margin: 0;\n  font-family: system-ui, sans-serif;\n  color: var(--text);\n}\n\n.card {\n  padding: 16px;\n  border-radius: 12px;\n  box-shadow: 0 2px 8px rgb(0 0 0 / 0.12);\n}\n\n.card:hover {\n  outline: 2px solid var(--accent);\n}\n\n@media (max-width: 600px) {\n  .card {\n    padding: 12px;\n  }\n}\n".to_string(),
        "js" => language
            .pick(
                "export function moyenne(valeurs) {\n  if (valeurs.length === 0) return 0;\n  return valeurs.reduce((a, b) => a + b, 0) / valeurs.length;\n}\n\nconst notes = [12, 18, 9];\nconsole.log(`Moyenne : ${moyenne(notes)}`);\n",
                "export function average(values) {\n  if (values.length === 0) return 0;\n  return values.reduce((a, b) => a + b, 0) / values.length;\n}\n\nconst scores = [12, 18, 9];\nconsole.log(`Average: ${average(scores)}`);\n",
            )
            .to_string(),
        "ts" => language
            .pick(
                "export interface Personne {\n  id: number;\n  prenom: string;\n  email: string;\n}\n\nexport const majeur = (age: number): boolean => age >= 18;\n\nexport function trouver(personnes: Personne[], id: number): Personne | undefined {\n  return personnes.find((personne) => personne.id === id);\n}\n",
                "export interface Person {\n  id: number;\n  firstName: string;\n  email: string;\n}\n\nexport const isAdult = (age: number): boolean => age >= 18;\n\nexport function find(people: Person[], id: number): Person | undefined {\n  return people.find((person) => person.id === id);\n}\n",
            )
            .to_string(),
        "py" => language
            .pick(
                "def moyenne(valeurs: list[float]) -> float:\n    \"\"\"La moyenne, ou 0 pour une liste vide.\"\"\"\n    return sum(valeurs) / len(valeurs) if valeurs else 0\n\n\nif __name__ == \"__main__\":\n    print(moyenne([12, 18, 9]))\n",
                "def average(values: list[float]) -> float:\n    \"\"\"The average, or 0 for an empty list.\"\"\"\n    return sum(values) / len(values) if values else 0\n\n\nif __name__ == \"__main__\":\n    print(average([12, 18, 9]))\n",
            )
            .to_string(),
        "sql" => sql(language),
        _ => return None,
    };
    Some(text)
}

fn log(rows: usize, language: Language) -> String {
    let messages = match language {
        Language::French => [
            ("INFO ", "Démarrage du service"),
            ("DEBUG", "Configuration lue (12 clés)"),
            ("INFO ", "Connexion à la base établie"),
            ("WARN ", "Cache vide, reconstruction"),
            ("INFO ", "Requête GET /api/personnes en 42 ms"),
            ("ERROR", "Délai dépassé vers example.com"),
        ],
        Language::English => [
            ("INFO ", "Service starting"),
            ("DEBUG", "Configuration read (12 keys)"),
            ("INFO ", "Connected to the database"),
            ("WARN ", "Cache empty, rebuilding"),
            ("INFO ", "Request GET /api/people in 42 ms"),
            ("ERROR", "Timed out reaching example.com"),
        ],
    };
    let mut text = String::new();
    for index in 0..rows {
        let millis = index * 1_337;
        let seconds = 9 * 3600 + millis / 1000;
        let (level, message) = messages[index % messages.len()];
        let _ = writeln!(
            text,
            "2026-01-15 {:02}:{:02}:{:02}.{:03} {level} {message}",
            (seconds / 3600) % 24,
            (seconds / 60) % 60,
            seconds % 60,
            millis % 1000
        );
    }
    text
}

fn sql(language: Language) -> String {
    let (table, first, city) = match language {
        Language::French => ("personnes", "prenom", "ville"),
        Language::English => ("people", "first_name", "city"),
    };
    let mut text = format!(
        "CREATE TABLE {table} (\n  id INTEGER PRIMARY KEY,\n  {first} TEXT NOT NULL,\n  email TEXT UNIQUE,\n  {city} TEXT\n);\n\nINSERT INTO {table} (id, {first}, email, {city}) VALUES\n"
    );
    let rows: Vec<String> = (0..5)
        .map(|index| {
            let p = Person::new(index, language);
            format!("  ({}, '{}', '{}', '{}')", p.id, p.first, p.email, p.city)
        })
        .collect();
    text += &rows.join(",\n");
    let _ = write!(
        text,
        ";\n\nSELECT {city}, COUNT(*) AS total\nFROM {table}\nGROUP BY {city}\nORDER BY total DESC;\n"
    );
    text
}

/// The first lines of a file, for the preview of the tool.
pub fn preview(content: &str) -> String {
    const LINES: usize = 14;
    let mut lines = content.lines();
    let mut shown: Vec<&str> = lines.by_ref().take(LINES).collect();
    if lines.next().is_some() {
        shown.push("…");
    }
    shown.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gives_the_rows_asked_for() {
        let csv = content("csv", 100, Language::French).unwrap();
        assert_eq!(csv.lines().count(), 101);
        assert!(csv.starts_with("id,prenom,nom,email,ville,inscription,montant\n1,Camille,"));
        assert_eq!(
            content("log", 10, Language::English)
                .unwrap()
                .lines()
                .count(),
            10
        );
    }

    #[test]
    fn writes_valid_json_with_unique_emails() {
        let json = content("json", 1000, Language::English).unwrap();
        let items: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(items.len(), 1000);
        let emails: std::collections::HashSet<_> =
            items.iter().map(|item| item["email"].clone()).collect();
        assert_eq!(emails.len(), 1000);
    }

    #[test]
    fn makes_emails_and_dates_plain() {
        let person = Person::new(2, Language::French);
        assert_eq!(person.first, "Léa");
        assert!(person.email.starts_with("lea."));
        assert_eq!(date_of_2026(0), "2026-01-01");
        assert_eq!(date_of_2026(31), "2026-02-01");
        assert_eq!(date_of_2026(364), "2026-12-31");
    }

    #[test]
    fn knows_every_text_format_and_nothing_else() {
        for format in [
            "txt", "md", "csv", "json", "xml", "yaml", "log", "html", "css", "js", "ts", "py",
            "sql",
        ] {
            assert!(content(format, 10, Language::French).is_some(), "{format}");
        }
        assert!(content("png", 10, Language::French).is_none());
    }

    #[test]
    fn previews_the_start() {
        let preview = preview(&content("csv", 100, Language::French).unwrap());
        assert_eq!(preview.lines().count(), 15);
        assert!(preview.ends_with('…'));
        assert!(!super::preview("a\nb").contains('…'));
    }
}
