use std::path::{Path, PathBuf};

use crate::state::BrowserLayout;

#[derive(Debug, Clone)]
pub struct Preferences {
    pub layout: BrowserLayout,
    pub show_hidden: bool,
    pub last_directory: Option<PathBuf>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            layout: BrowserLayout::Grid,
            show_hidden: false,
            last_directory: None,
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let Some(path) = preferences_path() else {
            return Self::default();
        };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };

        Self::parse(&contents)
    }

    fn parse(contents: &str) -> Self {
        let mut preferences = Self::default();
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "layout" => {
                    preferences.layout = if value == "list" {
                        BrowserLayout::List
                    } else {
                        BrowserLayout::Grid
                    };
                }
                "show_hidden" => preferences.show_hidden = value == "true",
                "last_directory" if !value.is_empty() => {
                    preferences.last_directory = Some(PathBuf::from(value));
                }
                _ => {}
            }
        }
        preferences
    }

    pub fn save(
        layout: BrowserLayout,
        show_hidden: bool,
        last_directory: &Path,
    ) -> std::io::Result<()> {
        let Some(path) = preferences_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let layout = match layout {
            BrowserLayout::Grid => "grid",
            BrowserLayout::List => "list",
        };
        let contents = format!(
            "layout={layout}\nshow_hidden={show_hidden}\nlast_directory={}\n",
            last_directory.to_string_lossy()
        );
        std::fs::write(path, contents)
    }
}

fn preferences_path() -> Option<PathBuf> {
    dirs::config_dir().map(|directory| directory.join("fotos").join("settings"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_persisted_preferences() {
        let preferences =
            Preferences::parse("layout=list\nshow_hidden=true\nlast_directory=/tmp/My Photos\n");

        assert_eq!(preferences.layout, BrowserLayout::List);
        assert!(preferences.show_hidden);
        assert_eq!(
            preferences.last_directory,
            Some(PathBuf::from("/tmp/My Photos"))
        );
    }
}
