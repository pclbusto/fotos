pub mod canvas;
pub mod io;
pub mod update;

pub use canvas::{CanvasState, ImageCanvasProgram};

pub fn scrollable_id() -> cosmic::iced::widget::Id {
    cosmic::iced::widget::Id::new("viewer-image-scrollable")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomMode {
    FitWindow,
    FitWidth,
    RealSize,
    Custom(f32),
}

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub handle: cosmic::iced::widget::image::Handle,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    pub file_size: Option<u64>,
    pub fields: Vec<(String, String)>,
}
