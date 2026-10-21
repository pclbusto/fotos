use cosmic::iced::widget::image::Handle;
use std::path::PathBuf;

pub mod io;
pub mod update;
pub mod view;

#[derive(Debug, Clone)]
pub enum BrowserItem {
    Image {
        name: String,
        path: PathBuf,
        file_size: Option<u64>,
        thumbnail: Option<Handle>,
    },
    Folder {
        name: String,
        path: PathBuf,
    },
}

impl BrowserItem {
    pub fn name(&self) -> &str {
        match self {
            BrowserItem::Image { name, .. } => name,
            BrowserItem::Folder { name, .. } => name,
        }
    }
}
