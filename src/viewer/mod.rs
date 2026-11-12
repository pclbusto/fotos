pub mod canvas;
pub mod io;
pub mod update;
pub mod zoom_filter;

pub use canvas::{CanvasState, ImageCanvasProgram};
pub use zoom_filter::{ZoomFilter, ZoomRenderState, ZOOM_FILTER_LABELS};

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
