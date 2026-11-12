//! WUp-only update exclusions, keyed by case-insensitive WinGet package ID.
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default)]
pub struct HiddenUpdates {
    packages: BTreeMap<String, (String, String)>,
}

impl HiddenUpdates {
    pub fn contains(&self, id: &str) -> bool {
        self.packages.contains_key(&id.to_lowercase())
    }

    pub fn insert(&mut self, id: &str, name: &str) {
        let clean = |value: &str| value.replace(['\t', '\r', '\n'], " ");
        self.packages
            .insert(id.to_lowercase(), (clean(id), clean(name)));
    }

    pub fn remove(&mut self, id: &str) {
        self.packages.remove(&id.to_lowercase());
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.packages.values()
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    fn decode(text: &str) -> Self {
        let mut result = Self::default();
        for line in text.lines() {
            if let Some((id, name)) = line.split_once('\t') {
                if !id.is_empty() {
                    result.insert(id, name);
                }
            }
        }
        result
    }

    fn encode(&self) -> String {
        self.iter()
            .map(|(id, name)| format!("{id}\t{name}\n"))
            .collect()
    }

    pub fn load() -> io::Result<Self> {
        Self::load_from(&Self::path()?)
    }

    fn load_from(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Self::decode(&text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::path()?)
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path.parent().unwrap())?;
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, self.encode())?;
        fs::rename(temporary, path)
    }

    fn path() -> io::Result<PathBuf> {
        std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .filter(|root| !root.is_empty())
            .map(|root| PathBuf::from(root).join("Wup").join("hidden-updates.tsv"))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "LOCALAPPDATA / APPDATA is unavailable",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_unicode_names_and_matches_ids_case_insensitively() {
        let mut hidden = HiddenUpdates::default();
        hidden.insert("Example.App", "예제 앱");
        hidden.insert("example.APP", "새 이름");
        let mut loaded = HiddenUpdates::decode(&hidden.encode());
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains("EXAMPLE.APP"));
        assert_eq!(loaded.iter().next().unwrap().1, "새 이름");
        loaded.remove("Example.App");
        assert_eq!(loaded.len(), 0);
    }

    #[test]
    fn skips_incomplete_records_and_prevents_extra_records_in_names() {
        let mut hidden = HiddenUpdates::decode("broken\n\tmissing ID\nExample.App\tApp\n");
        hidden.insert("Other.App", "Name\twith\nnewlines");
        let loaded = HiddenUpdates::decode(&hidden.encode());
        assert_eq!(loaded.len(), 2);
        assert!(loaded.contains("Other.App"));
    }

    #[test]
    fn saves_reloads_and_replaces_existing_preferences() {
        let root = std::env::temp_dir().join(format!(
            "wup-hidden-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("settings").join("hidden-updates.tsv");
        assert_eq!(HiddenUpdates::load_from(&path).unwrap().len(), 0);
        let mut hidden = HiddenUpdates::default();
        hidden.insert("Example.App", "예제 앱");
        hidden.save_to(&path).unwrap();
        assert!(
            HiddenUpdates::load_from(&path)
                .unwrap()
                .contains("example.app")
        );
        hidden.remove("Example.App");
        hidden.save_to(&path).unwrap();
        assert_eq!(HiddenUpdates::load_from(&path).unwrap().len(), 0);
        fs::remove_file(path).unwrap();
        fs::remove_dir(root.join("settings")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
