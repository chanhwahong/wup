//! Korean/English UI text and the persisted application language.
//! Raw package names and WinGet output are intentionally not translated.
use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Korean,
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Korean, Self::English];
    pub fn name(self) -> &'static str {
        match self {
            Self::Korean => "한국어",
            Self::English => "English",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Korean => "ko",
            Self::English => "en",
        }
    }
    pub fn t<'a>(self, korean: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::Korean => korean,
            Self::English => english,
        }
    }
    fn parse(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.key() == key.trim())
    }
    pub fn load() -> Self {
        preference_path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }
    pub fn save(self) -> io::Result<()> {
        let path = preference_path().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "LOCALAPPDATA / APPDATA is unavailable",
            )
        })?;
        self.save_to(&path)
    }
    fn load_from(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|key| Self::parse(&key))
            .unwrap_or_default()
    }
    fn save_to(self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("{}\n", self.key()))
    }
}

fn preference_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .filter(|path| !path.is_empty())
        .map(|root| PathBuf::from(root).join("Wup").join("language.txt"))
}

/// Store both translations so changing language also updates existing statuses,
/// errors and application-generated log entries without rerunning any operation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text {
    pub ko: String,
    pub en: String,
}

impl Text {
    pub fn new(ko: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            ko: ko.into(),
            en: en.into(),
        }
    }
    pub fn value(&self, language: Language) -> &str {
        language.t(&self.ko, &self.en)
    }
    pub fn limited(self, max_chars: usize) -> Self {
        Self {
            ko: self.ko.chars().take(max_chars).collect(),
            en: self.en.chars().take(max_chars).collect(),
        }
    }
}

impl From<String> for Text {
    fn from(raw: String) -> Self {
        Self {
            ko: raw.clone(),
            en: raw,
        }
    }
}
impl From<&str> for Text {
    fn from(raw: &str) -> Self {
        raw.to_owned().into()
    }
}
impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.en)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switches_existing_messages_and_preserves_raw_output() {
        let message = Text::new("업데이트 완료", "Update complete");
        assert_eq!(message.value(Language::Korean), "업데이트 완료");
        assert_eq!(message.value(Language::English), "Update complete");
        let raw = Text::from("WinGet: 한글 패키지 1.0 → 2.0");
        assert_eq!(raw.value(Language::Korean), raw.value(Language::English));
        assert_eq!(message.clone().limited(3).ko, "업데이");
    }
    #[test]
    fn stable_keys_and_persistence_with_safe_fallbacks() {
        assert_eq!(Language::parse(" en\n"), Some(Language::English));
        assert_eq!(Language::parse("ko"), Some(Language::Korean));
        assert_eq!(Language::parse("invalid"), None);
        let root = std::env::temp_dir().join(format!(
            "wup-language-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("settings").join("language.txt");
        assert_eq!(Language::load_from(&path), Language::Korean);
        for language in Language::ALL {
            language.save_to(&path).unwrap();
            assert_eq!(Language::load_from(&path), language);
        }
        fs::write(&path, "invalid").unwrap();
        assert_eq!(Language::load_from(&path), Language::Korean);
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert_eq!(Language::load_from(&path), Language::Korean);
        fs::remove_file(path).unwrap();
        fs::remove_dir(root.join("settings")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
