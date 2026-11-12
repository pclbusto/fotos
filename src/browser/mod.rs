use cosmic::iced::widget::image::Handle;
use std::path::{Path, PathBuf};

pub mod folders;
pub mod io;
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

    pub fn tags(&self) -> &[String] {
        match self {
            BrowserItem::Image { tags, .. } => tags,
        }
    }
}
