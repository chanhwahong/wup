//! Editor-inspired palettes and a small, dependency-free theme preference.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    CharcoalGreen,
    Dracula,
    OneDark,
    Nord,
    #[default]
    TokyoNight,
    GitHubLight,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: u32,
    pub panel: u32,
    pub surface: u32,
    pub log_background: u32,
    pub selection: u32,
    pub hover: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub success: u32,
    pub danger: u32,
    pub primary: u32,
    pub primary_hover: u32,
    pub primary_text: u32,
    pub disabled_text: u32,
    pub scroll_thumb: u32,
}

impl Theme {
    pub const ALL: [Self; 6] = [
        Self::CharcoalGreen,
        Self::Dracula,
        Self::OneDark,
        Self::Nord,
        Self::TokyoNight,
        Self::GitHubLight,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::CharcoalGreen => "Charcoal Green",
            Self::Dracula => "Dracula",
            Self::OneDark => "One Dark",
            Self::Nord => "Nord",
            Self::TokyoNight => "Tokyo Night",
            Self::GitHubLight => "GitHub Light",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::CharcoalGreen => "charcoal-green",
            Self::Dracula => "dracula",
            Self::OneDark => "one-dark",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo-night",
            Self::GitHubLight => "github-light",
        }
    }

    fn parse(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.key() == key.trim())
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::CharcoalGreen => Palette {
                background: 0x202222,
                panel: 0x242424,
                surface: 0x2b2b2b,
                log_background: 0x1e1e1e,
                selection: 0x3a3a3a,
                hover: 0x333333,
                border: 0x383838,
                text: 0xf3f3f3,
                muted: 0xb3b3b3,
                accent: 0x9be997,
                success: 0x9be997,
                danger: 0xff9b9b,
                primary: 0x347a3d,
                primary_hover: 0x398041,
                primary_text: 0xffffff,
                disabled_text: 0x757575,
                scroll_thumb: 0x6a6a6a,
            },
            Self::Dracula => Palette {
                background: 0x21222c,
                panel: 0x282a36,
                surface: 0x343746,
                log_background: 0x191a21,
                selection: 0x44475a,
                hover: 0x3b3e50,
                border: 0x44475a,
                text: 0xf8f8f2,
                muted: 0xb0aec7,
                accent: 0xbd93f9,
                success: 0x50fa7b,
                danger: 0xff808b,
                primary: 0xbd93f9,
                primary_hover: 0xd0afff,
                primary_text: 0x282a36,
                disabled_text: 0x787a90,
                scroll_thumb: 0x8d80ae,
            },
            Self::OneDark => Palette {
                background: 0x21252b,
                panel: 0x282c34,
                surface: 0x2c313c,
                log_background: 0x1c2026,
                selection: 0x323e54,
                hover: 0x333945,
                border: 0x3e4451,
                text: 0xabb2bf,
                muted: 0x969dab,
                accent: 0x61afef,
                success: 0x98c379,
                danger: 0xe88b92,
                primary: 0x61afef,
                primary_hover: 0x80c4f8,
                primary_text: 0x21252b,
                disabled_text: 0x656c78,
                scroll_thumb: 0x697383,
            },
            Self::Nord => Palette {
                background: 0x242933,
                panel: 0x2e3440,
                surface: 0x3b4252,
                log_background: 0x20252e,
                selection: 0x434c5e,
                hover: 0x3b4252,
                border: 0x4c566a,
                text: 0xeceff4,
                muted: 0xabb9cf,
                accent: 0x88c0d0,
                success: 0xa3be8c,
                danger: 0xe0949c,
                primary: 0x88c0d0,
                primary_hover: 0xa0d2e0,
                primary_text: 0x2e3440,
                disabled_text: 0x75839a,
                scroll_thumb: 0x7d8fa8,
            },
            Self::TokyoNight => Palette {
                background: 0x1a1b26,
                panel: 0x24283b,
                surface: 0x292e42,
                log_background: 0x16161e,
                selection: 0x33467c,
                hover: 0x30374f,
                border: 0x414868,
                text: 0xc0caf5,
                muted: 0x9aa5ce,
                accent: 0x7aa2f7,
                success: 0x9ece6a,
                danger: 0xf7768e,
                primary: 0x7aa2f7,
                primary_hover: 0x99b9ff,
                primary_text: 0x1a1b26,
                disabled_text: 0x697394,
                scroll_thumb: 0x737fa8,
            },
            Self::GitHubLight => Palette {
                background: 0xffffff,
                panel: 0xf6f8fa,
                surface: 0xeaeef2,
                log_background: 0xffffff,
                selection: 0xddf4ff,
                hover: 0xeaeef2,
                border: 0xd0d7de,
                text: 0x24292f,
                muted: 0x57606a,
                accent: 0x0969da,
                success: 0x1a7f37,
                danger: 0xcf222e,
                primary: 0x0969da,
                primary_hover: 0x0550ae,
                primary_text: 0xffffff,
                disabled_text: 0x8c959f,
                scroll_thumb: 0x6e7781,
            },
        }
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
        .map(|root| PathBuf::from(root).join("Wup").join("theme.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_has_a_unique_stable_key() {
        let keys: std::collections::HashSet<_> = Theme::ALL.into_iter().map(Theme::key).collect();
        assert_eq!(keys.len(), Theme::ALL.len());
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(&format!(" {}\n", theme.key())), Some(theme));
        }
        assert_eq!(Theme::parse("unknown"), None);
    }

    #[test]
    fn saves_reloads_and_recovers_from_invalid_preferences() {
        let root = std::env::temp_dir().join(format!(
            "wup-theme-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("settings").join("theme.txt");
        assert_eq!(Theme::load_from(&path), Theme::default());
        for theme in Theme::ALL {
            theme.save_to(&path).unwrap();
            assert_eq!(Theme::load_from(&path), theme);
        }
        fs::write(&path, "invalid-key").unwrap();
        assert_eq!(Theme::load_from(&path), Theme::default());
        fs::write(&path, [0xff, 0xfe]).unwrap();
        assert_eq!(Theme::load_from(&path), Theme::default());
        // Delete only the files and directories created by this test.
        fs::remove_file(path).unwrap();
        fs::remove_dir(root.join("settings")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    fn contrast(a: u32, b: u32) -> f64 {
        fn luminance(color: u32) -> f64 {
            [16, 8, 0]
                .into_iter()
                .zip([0.2126, 0.7152, 0.0722])
                .map(|(shift, weight)| {
                    let c = ((color >> shift) & 255) as f64 / 255.0;
                    weight
                        * if c <= 0.04045 {
                            c / 12.92
                        } else {
                            ((c + 0.055) / 1.055).powf(2.4)
                        }
                })
                .sum()
        }
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn body_and_primary_button_text_remain_readable_in_every_palette() {
        for theme in Theme::ALL {
            let p = theme.palette();
            for bg in [p.background, p.panel, p.selection, p.log_background] {
                assert!(
                    contrast(p.text, bg) >= 4.5,
                    "{} body contrast",
                    theme.name()
                );
            }
            assert!(
                contrast(p.primary_text, p.primary) >= 4.5,
                "{} primary contrast",
                theme.name()
            );
            assert!(
                contrast(p.muted, p.panel) >= 4.5,
                "{} muted contrast",
                theme.name()
            );
        }
    }
}
