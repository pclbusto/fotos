pub mod canvas;
pub mod io;
pub mod update;

pub use canvas::{CanvasState, ImageCanvasProgram};

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
