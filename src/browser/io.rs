use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{collections::hash_map::DefaultHasher, hash::Hash, hash::Hasher};

use cosmic::iced::Task;
use tokio::sync::Semaphore;

use crate::browser::BrowserItem;
use crate::codecs;
use crate::editor;
use crate::message::{BrowserMessage, Message};

fn thumbnail_semaphore() -> &'static Semaphore {
    static SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();
    SEMAPHORE.get_or_init(|| Semaphore::new(4))
}

pub fn pictures_dir() -> PathBuf {
    if let Some(pictures) = dirs::picture_dir() {
        if pictures.exists() {
            return pictures;
        }
    }
    if let Some(home) = dirs::home_dir() {
        let pictures = home.join("Pictures");
        if pictures.exists() {
            return pictures;
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

pub fn load_directory(dir: PathBuf, show_hidden: bool) -> Result<Vec<BrowserItem>, String> {
    let mut items = Vec::new();
    if let Some(parent) = dir.parent() {
        if parent != dir {
            items.push(BrowserItem::Folder {
                name: "..".to_string(),
                path: parent.to_path_buf(),
            });
        }
    }

    let entries = std::fs::read_dir(&dir).map_err(|error| error.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        if !show_hidden && name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            items.push(BrowserItem::Folder { name, path });
        } else if path.is_file() && codecs::supports(&path) {
            let file_size = entry.metadata().ok().map(|metadata| metadata.len());
            items.push(BrowserItem::Image {
                name,
                path,
                file_size,
                thumbnail: None,
            });
        }
    }

    items.sort_by(|first, second| {
        let first_dir = matches!(first, BrowserItem::Folder { .. });
        let second_dir = matches!(second, BrowserItem::Folder { .. });
        if first_dir == second_dir {
            first
                .name()
                .to_lowercase()
                .cmp(&second.name().to_lowercase())
        } else if first_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    Ok(items)
}

fn load_thumbnail(path: &Path) -> Option<cosmic::iced::widget::image::Handle> {
    let cached_path = thumbnail_cache_path(path);
    let cache_is_current = cached_path.as_ref().is_some_and(|cached| {
        let source_modified = std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok();
        let cached_modified = std::fs::metadata(cached)
            .and_then(|metadata| metadata.modified())
            .ok();
        matches!((source_modified, cached_modified), (Some(source), Some(cache)) if cache >= source)
    });

    if cache_is_current {
        if let Some(image) = cached_path.as_ref().and_then(|path| image::open(path).ok()) {
            return Some(editor::to_handle(&image));
        }
    }

    let thumbnail = codecs::open(path).ok()?.thumbnail(120, 120);
    if let Some(cached) = cached_path {
        if let Some(parent) = cached.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = thumbnail.save(cached);
    }
    Some(editor::to_handle(&thumbnail))
}

fn thumbnail_cache_path(path: &Path) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let name = format!("{:016x}.png", hasher.finish());
    dirs::cache_dir().map(|directory| directory.join("fotos").join("thumbnails").join(name))
}

pub fn thumbnail_task(
    index: usize,
    path: PathBuf,
    generation: usize,
) -> Task<cosmic::Action<Message>> {
    let thumbnail_path = path.clone();
    Task::perform(
        async move {
            let _permit = thumbnail_semaphore().acquire().await;
            tokio::task::spawn_blocking(move || load_thumbnail(&thumbnail_path))
                .await
                .ok()
                .flatten()
        },
        move |handle| {
            cosmic::Action::App(
                BrowserMessage::ThumbnailLoaded {
                    index,
                    path: path.clone(),
                    handle,
                    generation,
                }
                .into(),
            )
        },
    )
}
