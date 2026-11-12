use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::codecs;

/// Carpetas de caché, sistema o desarrollo que se omiten siempre en el escaneo recursivo del Home.
const IGNORED_DIRS: &[&str] = &[
    ".cache",
    ".local",
    ".config",
    ".cargo",
    ".rustup",
    ".git",
    ".hg",
    ".svn",
    ".npm",
    ".pnpm-store",
    ".yarn",
    ".gradle",
    ".m2",
    ".ivy2",
    ".vscode",
    ".idea",
    ".mozilla",
    ".thunderbird",
    ".steam",
    ".var",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    "Trash",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRecord {
    pub path: PathBuf,
    pub folder: PathBuf,
    pub name: String,
    pub file_size: u64,
    pub modified_secs: u64,
    pub tags: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanStats {
    pub total_indexed: usize,
    pub added: usize,
    pub removed: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DatabaseFilter {
    pub selected_folders: BTreeSet<PathBuf>,
    pub selected_tags: BTreeSet<String>,
    pub untagged_only: bool,
    pub search_query: String,
    pub show_hidden: bool,
}

impl DatabaseFilter {
    pub fn is_active(&self) -> bool {
        !self.selected_folders.is_empty()
            || !self.selected_tags.is_empty()
            || self.untagged_only
            || !self.search_query.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageDatabase {
    records: BTreeMap<PathBuf, ImageRecord>,
}

impl ImageDatabase {
    pub fn load() -> Self {
        let Some(path) = database_path() else {
            return Self::default();
        };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self::parse(&contents)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = database_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.serialize())
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn get(&self, path: &Path) -> Option<&ImageRecord> {
        self.records.get(path)
    }

    pub fn tags_for(&self, path: &Path) -> Vec<String> {
        self.records
            .get(path)
            .map(|record| record.tags.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn add_tag(&mut self, path: &Path, raw_tag: &str) -> bool {
        let tag = normalize_tag(raw_tag);
        if tag.is_empty() {
            return false;
        }
        if !self.records.contains_key(path) {
            self.upsert_single_file(path);
        }
        if let Some(record) = self.records.get_mut(path) {
            return record.tags.insert(tag);
        }
        false
    }

    pub fn remove_tag(&mut self, path: &Path, tag: &str) -> bool {
        if let Some(record) = self.records.get_mut(path) {
            return record.tags.remove(tag);
        }
        false
    }

    pub fn toggle_tag(&mut self, path: &Path, raw_tag: &str) -> bool {
        let tag = normalize_tag(raw_tag);
        if tag.is_empty() {
            return false;
        }
        if !self.records.contains_key(path) {
            self.upsert_single_file(path);
        }
        if let Some(record) = self.records.get_mut(path) {
            if record.tags.contains(&tag) {
                record.tags.remove(&tag);
            } else {
                record.tags.insert(tag);
            }
            return true;
        }
        false
    }

    /// Quita todos los registros dentro de `root` (incluida la propia carpeta).
    pub fn remove_under(&mut self, root: &Path) -> usize {
        let before = self.records.len();
        self.records.retain(|path, _| !path.starts_with(root));
        before - self.records.len()
    }

    /// Quita los registros que caen dentro de alguna carpeta ignorada.
    pub fn remove_ignored(&mut self, ignored: &IgnoredFolders) -> usize {
        let before = self.records.len();
        self.records.retain(|path, _| !ignored.contains_path(path));
        before - self.records.len()
    }

    pub fn remove_path(&mut self, path: &Path) -> bool {
        self.records.remove(path).is_some()
    }

    pub fn upsert_single_file(&mut self, path: &Path) -> bool {
        if !path.is_file() || !codecs::supports(path) {
            return false;
        }
        let Some(scanned) = inspect_image_file(path) else {
            return false;
        };
        if let Some(existing) = self.records.get_mut(path) {
            existing.file_size = scanned.file_size;
            existing.modified_secs = scanned.modified_secs;
            existing.name = scanned.name;
            existing.folder = scanned.folder;
        } else {
            self.records.insert(path.to_path_buf(), scanned);
        }
        true
    }

    /// Escanea únicamente la carpeta indicada (sin entrar en subcarpetas) y sincroniza sus imágenes en la BD.
    pub fn scan_directory(&mut self, dir: &Path, show_hidden: bool) -> Result<ScanStats, String> {
        let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
        let mut found_paths = BTreeSet::new();
        let mut added = 0;

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            if path.is_file() && codecs::supports(&path) {
                if let Some(scanned) = inspect_image_file(&path) {
                    found_paths.insert(path.clone());
                    if let Some(existing) = self.records.get_mut(&path) {
                        existing.file_size = scanned.file_size;
                        existing.modified_secs = scanned.modified_secs;
                        existing.name = scanned.name;
                        existing.folder = scanned.folder;
                    } else {
                        self.records.insert(path, scanned);
                        added += 1;
                    }
                }
            }
        }

        // Eliminar registros de esta misma carpeta que ya no existen en el disco
        let stale: Vec<PathBuf> = self
            .records
            .iter()
            .filter(|(path, record)| {
                record.folder == dir && (!found_paths.contains(*path) && !path.exists())
            })
            .map(|(path, _)| path.clone())
            .collect();

        let removed = stale.len();
        for path in stale {
            self.records.remove(&path);
        }

        Ok(ScanStats {
            total_indexed: found_paths.len(),
            added,
            removed,
        })
    }

    /// Escanea agresivamente todo el directorio raíz (`home`) de forma recursiva.
    pub fn scan_home_recursive(
        &mut self,
        root: &Path,
        show_hidden: bool,
        ignored: &IgnoredFolders,
    ) -> Result<ScanStats, String> {
        if !root.is_dir() {
            return Err(format!(
                "El directorio {} no existe",
                root.to_string_lossy()
            ));
        }

        let mut stack = vec![root.to_path_buf()];
        let mut found_paths = BTreeSet::new();
        let mut added = 0;

        while let Some(current_dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&current_dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                // Evitar ciclos por enlaces simbólicos
                if file_type.is_symlink() {
                    continue;
                }

                let path = entry.path();
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());

                if file_type.is_dir() {
                    if should_ignore_dir(&name, show_hidden) || ignored.contains(&path) {
                        continue;
                    }
                    stack.push(path);
                } else if file_type.is_file() {
                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }
                    if codecs::supports(&path) {
                        if let Some(scanned) = inspect_image_file(&path) {
                            found_paths.insert(path.clone());
                            if let Some(existing) = self.records.get_mut(&path) {
                                existing.file_size = scanned.file_size;
                                existing.modified_secs = scanned.modified_secs;
                                existing.name = scanned.name;
                                existing.folder = scanned.folder;
                            } else {
                                self.records.insert(path, scanned);
                                added += 1;
                            }
                        }
                    }
                }
            }
        }

        // Limpiar registros bajo `root` cuyos archivos ya fueron borrados del disco
        let stale: Vec<PathBuf> = self
            .records
            .keys()
            .filter(|path| {
                path.starts_with(root) && !found_paths.contains(*path) && !path.exists()
            })
            .cloned()
            .collect();

        let removed = stale.len();
        for path in stale {
            self.records.remove(&path);
        }

        Ok(ScanStats {
            total_indexed: found_paths.len(),
            added,
            removed,
        })
    }

    /// Lista todas las carpetas presentes en la base de datos junto con la cantidad de imágenes en cada una.
    pub fn all_folders(&self, show_hidden: bool) -> Vec<(PathBuf, usize)> {
        let mut counts: BTreeMap<PathBuf, usize> = BTreeMap::new();
        for record in self.records.values() {
            if !show_hidden && record.name.starts_with('.') {
                continue;
            }
            *counts.entry(record.folder.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// Lista todas las etiquetas presentes en la base de datos junto con cuántas imágenes las usan.
    pub fn all_tags(&self, show_hidden: bool) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for record in self.records.values() {
            if !show_hidden && record.name.starts_with('.') {
                continue;
            }
            for tag in &record.tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        counts.into_iter().collect()
    }

    pub fn untagged_count(&self, show_hidden: bool) -> usize {
        self.records
            .values()
            .filter(|record| {
                (show_hidden || !record.name.starts_with('.')) && record.tags.is_empty()
            })
            .count()
    }

    /// Filtra la bolsa de imágenes según las carpetas seleccionadas, etiquetas y búsqueda de texto.
    pub fn query(&self, filter: &DatabaseFilter) -> Vec<ImageRecord> {
        let query_lower = filter.search_query.trim().to_lowercase();

        let mut results: Vec<ImageRecord> = self
            .records
            .values()
            .filter(|record| {
                if !filter.show_hidden && record.name.starts_with('.') {
                    return false;
                }
                if !filter.selected_folders.is_empty()
                    && !filter.selected_folders.contains(&record.folder)
                {
                    return false;
                }
                if filter.untagged_only && !record.tags.is_empty() {
                    return false;
                }
                if !filter.selected_tags.is_empty()
                    && !filter
                        .selected_tags
                        .iter()
                        .all(|wanted| record.tags.contains(wanted))
                {
                    return false;
                }
                if !query_lower.is_empty() {
                    let matches_name = record.name.to_lowercase().contains(&query_lower);
                    let matches_folder = record
                        .folder
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query_lower);
                    let matches_tag = record
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower));
                    if !matches_name && !matches_folder && !matches_tag {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.path.cmp(&b.path))
        });

        results
    }

    pub fn parse(contents: &str) -> Self {
        let mut records = BTreeMap::new();
        for line in contents.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.splitn(4, '\t');
            let Some(path_str) = parts.next() else {
                continue;
            };
            let Some(size_str) = parts.next() else {
                continue;
            };
            let Some(modified_str) = parts.next() else {
                continue;
            };
            let tags_str = parts.next().unwrap_or("");

            let path = PathBuf::from(unescape_field(path_str));
            let folder = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("/"));
            let name = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            let file_size = size_str.parse::<u64>().unwrap_or(0);
            let modified_secs = modified_str.parse::<u64>().unwrap_or(0);
            let tags: BTreeSet<String> = tags_str
                .split(',')
                .map(unescape_tag)
                .map(|t| normalize_tag(&t))
                .filter(|t| !t.is_empty())
                .collect();

            records.insert(
                path.clone(),
                ImageRecord {
                    path,
                    folder,
                    name,
                    file_size,
                    modified_secs,
                    tags,
                },
            );
        }
        Self { records }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from("# fotos image database v1\n");
        for record in self.records.values() {
            let escaped_path = escape_field(&record.path.to_string_lossy());
            let tags_joined = record
                .tags
                .iter()
                .map(|t| escape_tag(t))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(
                "{escaped_path}\t{}\t{}\t{tags_joined}\n",
                record.file_size, record.modified_secs
            ));
        }
        out
    }
}

/// Carpetas que el usuario eligió ignorar: no se escanean ni se muestran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoredFolders {
    folders: BTreeSet<PathBuf>,
}

impl IgnoredFolders {
    pub fn load() -> Self {
        ignored_folders_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|contents| Self::parse(&contents))
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = ignored_folders_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.serialize())
    }

    fn parse(contents: &str) -> Self {
        Self {
            folders: contents
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                .map(|line| PathBuf::from(unescape_field(line)))
                .collect(),
        }
    }

    fn serialize(&self) -> String {
        let mut out = String::from("# Carpetas ignoradas por Fotos, una por línea\n");
        for folder in &self.folders {
            out.push_str(&escape_field(&folder.to_string_lossy()));
            out.push('\n');
        }
        out
    }

    /// `true` si la carpeta está ignorada exactamente.
    pub fn contains(&self, folder: &Path) -> bool {
        self.folders.contains(folder)
    }

    /// `true` si `path` es una carpeta ignorada o está dentro de una.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.folders.iter().any(|folder| path.starts_with(folder))
    }

    pub fn insert(&mut self, folder: PathBuf) -> bool {
        self.folders.insert(folder)
    }

    pub fn remove(&mut self, folder: &Path) -> bool {
        self.folders.remove(folder)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.folders.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
    }
}

fn ignored_folders_path() -> Option<PathBuf> {
    dirs::config_dir().map(|directory| directory.join("fotos").join("ignored_folders"))
}

fn should_ignore_dir(name: &str, show_hidden: bool) -> bool {
    if IGNORED_DIRS.contains(&name) {
        return true;
    }
    if !show_hidden && name.starts_with('.') {
        return true;
    }
    false
}

fn inspect_image_file(path: &Path) -> Option<ImageRecord> {
    let metadata = std::fs::metadata(path).ok()?;
    let file_size = metadata.len();
    let modified_secs = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let folder = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/"));
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());

    Some(ImageRecord {
        path: path.to_path_buf(),
        folder,
        name,
        file_size,
        modified_secs,
        tags: BTreeSet::new(),
    })
}

fn normalize_tag(raw: &str) -> String {
    raw.trim().replace(['\t', '\n', '\r'], " ")
}

fn escape_field(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_field(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape_tag(s: &str) -> String {
    escape_field(s).replace(',', "\\c")
}

fn unescape_tag(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('c') => out.push(','),
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn database_path() -> Option<PathBuf> {
    dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .map(|directory| directory.join("fotos").join("database.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_database_records_and_tags() {
        let mut db = ImageDatabase::default();
        let path = PathBuf::from("/home/user/Pictures/Vacaciones 2026/playa.png");
        db.records.insert(
            path.clone(),
            ImageRecord {
                path: path.clone(),
                folder: PathBuf::from("/home/user/Pictures/Vacaciones 2026"),
                name: "playa.png".to_string(),
                file_size: 4096,
                modified_secs: 1700000000,
                tags: BTreeSet::from(["familia".to_string(), "verano, mar".to_string()]),
            },
        );

        let serialized = db.serialize();
        let loaded = ImageDatabase::parse(&serialized);
        let record = loaded.get(&path).expect("record should exist");
        assert_eq!(record.file_size, 4096);
        assert_eq!(record.modified_secs, 1700000000);
        assert!(record.tags.contains("familia"));
        assert!(record.tags.contains("verano, mar"));
    }

    #[test]
    fn filters_by_folder_and_tags_and_search() {
        let mut db = ImageDatabase::default();
        let p1 = PathBuf::from("/home/user/Pictures/a.png");
        let p2 = PathBuf::from("/home/user/Downloads/b.jpg");

        db.records.insert(
            p1.clone(),
            ImageRecord {
                path: p1.clone(),
                folder: PathBuf::from("/home/user/Pictures"),
                name: "a.png".to_string(),
                file_size: 100,
                modified_secs: 1,
                tags: BTreeSet::from(["favoritos".to_string(), "viaje".to_string()]),
            },
        );
        db.records.insert(
            p2.clone(),
            ImageRecord {
                path: p2.clone(),
                folder: PathBuf::from("/home/user/Downloads"),
                name: "b.jpg".to_string(),
                file_size: 200,
                modified_secs: 2,
                tags: BTreeSet::from(["trabajo".to_string()]),
            },
        );

        let mut filter = DatabaseFilter {
            show_hidden: false,
            ..Default::default()
        };
        assert_eq!(db.query(&filter).len(), 2);

        filter
            .selected_folders
            .insert(PathBuf::from("/home/user/Pictures"));
        let by_folder = db.query(&filter);
        assert_eq!(by_folder.len(), 1);
        assert_eq!(by_folder[0].path, p1);

        filter.selected_folders.clear();
        filter.selected_tags.insert("trabajo".to_string());
        let by_tag = db.query(&filter);
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].path, p2);
    }

    #[test]
    fn ignored_folders_round_trip_and_filter() {
        let mut ignored = IgnoredFolders::default();
        ignored.insert(PathBuf::from("/h/Android"));
        ignored.insert(PathBuf::from("/h/con\ttab"));
        let parsed = IgnoredFolders::parse(&ignored.serialize());
        assert_eq!(parsed, ignored);

        assert!(ignored.contains_path(Path::new("/h/Android/Sdk/icon.png")));
        assert!(!ignored.contains_path(Path::new("/h/AndroidX/a.png")));

        let mut db = ImageDatabase::parse(
            "/h/Android/Sdk/a.png\t1\t1\t\n/h/Fotos/b.png\t1\t1\t\n/h/AndroidX/c.png\t1\t1\t\n",
        );
        assert_eq!(db.remove_ignored(&ignored), 1);
        assert_eq!(db.len(), 2);
        assert_eq!(db.remove_under(Path::new("/h/Fotos")), 1);
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn scan_skips_ignored_folders() {
        let root = std::env::temp_dir().join(format!("fotos-scan-test-{}", std::process::id()));
        let keep = root.join("fotos");
        let skip = root.join("ignorada");
        std::fs::create_dir_all(&keep).unwrap();
        std::fs::create_dir_all(&skip).unwrap();
        image::RgbImage::new(2, 2).save(keep.join("a.png")).unwrap();
        image::RgbImage::new(2, 2).save(skip.join("b.png")).unwrap();

        let mut ignored = IgnoredFolders::default();
        ignored.insert(skip.clone());
        let mut db = ImageDatabase::default();
        let stats = db.scan_home_recursive(&root, false, &ignored).unwrap();
        assert_eq!(stats.total_indexed, 1);
        assert!(db.get(&keep.join("a.png")).is_some());
        assert!(db.get(&skip.join("b.png")).is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
