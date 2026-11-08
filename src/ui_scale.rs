//! Application UI zoom, independent of the Windows display DPI setting.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiScale(u16);

impl Default for UiScale {
    fn default() -> Self {
        Self(100)
    }
}

impl UiScale {
    pub const MIN: u16 = 80;
    pub const MAX: u16 = 120;
    pub const STEP: u16 = 5;
    pub fn new(percent: u16) -> Self {
        Self(percent.clamp(Self::MIN, Self::MAX))
    }
    pub fn percent(self) -> u16 {
        self.0
    }
    pub fn factor(self) -> f32 {
        // New 100% is the original application's 90% size.
        self.0 as f32 * 9.0 / 1000.0
    }
    pub fn smaller(self) -> Self {
        Self::new(self.0.saturating_sub(Self::STEP))
    }
    pub fn larger(self) -> Self {
        Self::new(self.0.saturating_add(Self::STEP))
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
            .and_then(|s| Self::parse(&s))
            .unwrap_or_default()
    }

    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(current) = text.strip_prefix("v2:") {
            return current.parse::<u16>().ok().map(Self::new);
        }
        // Legacy percentages used the original 100% baseline. Convert to the
        // nearest 5% step on the new baseline (old 90% -> new 100%).
        text.parse::<u16>().ok().map(|old| {
            let converted = ((u32::from(old) * 2 + 4) / 9 * 5).min(u32::from(Self::MAX));
            Self::new(converted as u16)
        })
    }
    fn save_to(self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, format!("v2:{}\n", self.percent()))
    }
}

fn preference_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .filter(|path| !path.is_empty())
        .map(|root| PathBuf::from(root).join("Wup").join("scale.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_is_bounded_and_steps_without_rounding_drift() {
        assert_eq!(UiScale::default().factor(), 0.9);
        assert_eq!(UiScale::new(0).percent(), 80);
        assert_eq!(UiScale::new(u16::MAX).percent(), 120);
        let mut scale = UiScale::default();
        for _ in 0..30 {
            scale = scale.smaller();
        }
        assert_eq!(scale.factor(), 0.72);
        for _ in 0..30 {
            scale = scale.larger();
        }
        assert_eq!(scale.factor(), 1.08);
        for _ in 0..4 {
            scale = scale.smaller();
        }
        assert_eq!(scale, UiScale::default());
    }

    #[test]
    fn legacy_90_percent_becomes_100_without_reconverting_new_settings() {
        assert_eq!(UiScale::parse("90\n"), Some(UiScale::new(100)));
        assert_eq!(UiScale::parse("100"), Some(UiScale::new(110)));
        assert_eq!(UiScale::parse("80"), Some(UiScale::new(90)));
        assert_eq!(UiScale::parse("v2:90"), Some(UiScale::new(90)));
        assert_eq!(UiScale::parse("v2:100"), Some(UiScale::new(100)));
    }
    #[test]
    fn preferences_round_trip_clamp_and_recover() {
        let root = std::env::temp_dir().join(format!(
            "wup-scale-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("settings").join("scale.txt");
        assert_eq!(UiScale::load_from(&path), UiScale::default());
        for percent in (80..=120).step_by(5) {
            let scale = UiScale::new(percent);
            scale.save_to(&path).unwrap();
            assert_eq!(UiScale::load_from(&path), scale);
        }
        for (text, expected) in [
            ("200", 120),
            ("1", 80),
            ("bad", 100),
            ("", 100),
            ("-1", 100),
        ] {
            fs::write(&path, text).unwrap();
            assert_eq!(UiScale::load_from(&path).percent(), expected);
        }
        fs::remove_file(path).unwrap();
        fs::remove_dir(root.join("settings")).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
