use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub const BUILT_IN_PALETTE: &str = "built-in";
pub const CUSTOM_PALETTE: &str = "custom";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnsiPalette {
    pub name: String,
    pub colors: [u32; 16],
}

#[derive(Debug, Default)]
pub struct AnsiPaletteCatalog {
    palettes: Vec<AnsiPalette>,
    errors: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnsiPalette {
    colors: [String; 16],
}

impl AnsiPalette {
    fn parse(name: String, json: &str) -> Result<Self, String> {
        let raw: RawAnsiPalette =
            serde_json::from_str(json).map_err(|error| format!("invalid palette JSON: {error}"))?;
        let mut colors = [0; 16];
        for (target, value) in colors.iter_mut().zip(raw.colors) {
            let error = || format!("{value}: expected #RRGGBB");
            let digits = value.strip_prefix('#').ok_or_else(error)?;
            if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(error());
            }
            *target = u32::from_str_radix(digits, 16).map_err(|_| error())?;
        }
        Ok(Self { name, colors })
    }
}

impl AnsiPaletteCatalog {
    pub fn load(directory: &Path) -> Self {
        let Ok(entries) = fs::read_dir(directory) else {
            return Self::default();
        };
        let mut catalog = Self::default();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            if matches!(name, BUILT_IN_PALETTE | CUSTOM_PALETTE) {
                continue;
            }
            let result = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))
                .and_then(|json| AnsiPalette::parse(name.to_owned(), &json));
            match result {
                Ok(palette) => catalog.palettes.push(palette),
                Err(error) => {
                    catalog.errors.insert(name.to_owned(), error);
                }
            }
        }
        catalog
            .palettes
            .sort_by_key(|palette| palette.name.to_lowercase());
        catalog
    }

    pub fn palettes(&self) -> &[AnsiPalette] {
        &self.palettes
    }

    pub fn get(&self, name: &str) -> Option<&AnsiPalette> {
        self.palettes.iter().find(|palette| palette.name == name)
    }

    pub fn error(&self, name: &str) -> Option<&str> {
        if self.get(name).is_some() {
            None
        } else {
            Some(
                self.errors
                    .get(name)
                    .map(String::as_str)
                    .unwrap_or("palette file is missing or invalid"),
            )
        }
    }
}

pub fn palettes_directory() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let parent = executable
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_owned())?;
    Ok(parent.join("palettes"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::AnsiPaletteCatalog;

    fn palette_json(color: &str) -> String {
        format!(
            r#"{{"colors":[{}]}}"#,
            std::iter::repeat_n(format!(r#""{color}""#), 16)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    #[test]
    fn palette_catalog_loads_valid_files_once_in_name_order() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("zeta.json"), palette_json("#112233")).unwrap();
        fs::write(directory.path().join("Alpha.json"), palette_json("#A1B2C3")).unwrap();
        fs::write(directory.path().join("invalid.json"), palette_json("red")).unwrap();
        fs::write(
            directory.path().join("short.json"),
            palette_json("#112233").replace(",\"#112233\"", ""),
        )
        .unwrap();
        fs::write(
            directory.path().join("built-in.json"),
            palette_json("#000000"),
        )
        .unwrap();
        fs::write(
            directory.path().join("custom.json"),
            palette_json("#000000"),
        )
        .unwrap();
        fs::write(directory.path().join("notes.txt"), palette_json("#000000")).unwrap();

        let catalog = AnsiPaletteCatalog::load(directory.path());
        let names = catalog
            .palettes()
            .iter()
            .map(|palette| palette.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, ["Alpha", "zeta"]);
        assert_eq!(catalog.get("Alpha").unwrap().colors, [0xa1b2c3; 16]);
        assert!(catalog.error("invalid").is_some());
        assert!(catalog.error("missing").is_some());
        assert_eq!(catalog.error("Alpha"), None);

        fs::write(directory.path().join("Alpha.json"), palette_json("#ffffff")).unwrap();
        fs::write(directory.path().join("later.json"), palette_json("#000000")).unwrap();
        assert_eq!(catalog.get("Alpha").unwrap().colors, [0xa1b2c3; 16]);
        assert!(catalog.get("later").is_none());
    }
}
