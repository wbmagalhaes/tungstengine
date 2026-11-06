use std::collections::HashMap;
use std::path::Path;

use crate::ecs::Resource;

#[derive(Resource, Clone, Debug)]
pub struct Localization {
    locale: String,
    fallback: String,
    tables: HashMap<String, HashMap<String, String>>,
}

impl Default for Localization {
    fn default() -> Self {
        Localization::new("en")
    }
}

impl Localization {
    pub fn new(fallback: &str) -> Self {
        Localization {
            locale: fallback.to_string(),
            fallback: fallback.to_string(),
            tables: HashMap::new(),
        }
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
    }

    pub fn locales(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.tables.keys().map(String::as_str).collect();
        v.sort();
        v
    }

    pub fn load_str(&mut self, locale: &str, text: &str) -> Result<(), String> {
        let value: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
        let table = self.tables.entry(locale.to_string()).or_default();
        flatten("", &value, table);
        Ok(())
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(locale) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            self.load_str(locale, &text)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    }

    pub fn language_name(&self, locale: &str) -> String {
        if let Some(name) = self.tables.get(locale).and_then(|t| t.get("language.name")) {
            return name.to_string();
        }
        match locale {
            "pt-BR" => "Português".to_string(),
            "en-US" => "English".to_string(),
            other => other.to_string(),
        }
    }

    pub fn has(&self, key: &str) -> bool {
        self.lookup(key).is_some()
    }

    pub fn t(&self, key: &str) -> String {
        self.lookup(key).unwrap_or(key).to_string()
    }

    pub fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        interpolate(self.lookup(key).unwrap_or(key), args)
    }

    pub fn tn(&self, key: &str, count: i64, args: &[(&str, &str)]) -> String {
        let form = if count == 1 { "one" } else { "other" };
        let plural = format!("{key}.{form}");
        let text = self.lookup(&plural).or_else(|| self.lookup(key));
        let count = count.to_string();
        let mut all = vec![("count", count.as_str())];
        all.extend_from_slice(args);
        interpolate(text.unwrap_or(key), &all)
    }

    fn lookup(&self, key: &str) -> Option<&str> {
        [&self.locale, &self.fallback]
            .into_iter()
            .find_map(|l| self.tables.get(l)?.get(key))
            .map(String::as_str)
    }
}

fn flatten(prefix: &str, table: &toml::Table, out: &mut HashMap<String, String>) {
    for (k, v) in table {
        let key = if prefix.is_empty() {
            k.clone()
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::String(s) => {
                out.insert(key, s.clone());
            }
            toml::Value::Table(t) => flatten(&key, t, out),
            _ => log::warn!("localization: `{key}` is not a string"),
        }
    }
}

fn interpolate(text: &str, args: &[(&str, &str)]) -> String {
    args.iter().fold(text.to_string(), |s, (k, v)| {
        s.replace(&format!("{{{k}}}"), v)
    })
}

pub mod prelude {
    pub use super::Localization;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> Localization {
        let mut l = Localization::new("en");
        l.load_str(
            "en",
            "[menu]\nplay = \"Play\"\nquit = \"Quit\"\nhello = \"Hi {name}\"\n[items]\none = \"{count} item\"\nother = \"{count} items\"\n",
        )
        .unwrap();
        l.load_str("pt-BR", "[menu]\nplay = \"Jogar\"\n").unwrap();
        l
    }

    #[test]
    fn uses_locale_then_fallback_then_key() {
        let mut l = loc();
        l.set_locale("pt-BR");
        assert_eq!(l.t("menu.play"), "Jogar");
        assert_eq!(l.t("menu.quit"), "Quit");
        assert_eq!(l.t("menu.nope"), "menu.nope");
    }

    #[test]
    fn interpolates_and_pluralizes() {
        let l = loc();
        assert_eq!(l.tf("menu.hello", &[("name", "Nox")]), "Hi Nox");
        assert_eq!(l.tn("items", 1, &[]), "1 item");
        assert_eq!(l.tn("items", 3, &[]), "3 items");
    }
}
