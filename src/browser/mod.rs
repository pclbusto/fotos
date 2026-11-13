use cosmic::iced::widget::image::Handle;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub mod folders;
pub mod io;
pub mod sort;
pub mod thumbnails;
pub mod update;
pub mod view;

#[derive(Debug, Clone)]
pub enum BrowserItem {
    Image {
        name: String,
        path: PathBuf,
        folder: PathBuf,
        file_size: Option<u64>,
        modified: Option<SystemTime>,
        tags: Vec<String>,
        thumbnail: Option<Handle>,
    },
}

impl BrowserItem {
    pub fn name(&self) -> &str {
        match self {
            BrowserItem::Image { name, .. } => name,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            BrowserItem::Image { path, .. } => path,
        }
    }

    pub fn folder(&self) -> &Path {
        match self {
            BrowserItem::Image { folder, .. } => folder,
        }
    }

    pub fn file_size(&self) -> Option<u64> {
        match self {
            BrowserItem::Image { file_size, .. } => *file_size,
        }
    }

    pub fn modified(&self) -> Option<SystemTime> {
        match self {
            BrowserItem::Image { modified, .. } => *modified,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            BrowserItem::Image { tags, .. } => tags,
        }
    }
}
