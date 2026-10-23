use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::widget::image::Handle;
use cosmic::iced::Point;
use cosmic::widget::about::About;
use image::DynamicImage;

use crate::browser::BrowserItem;
use crate::viewer::{CachedImage, ImageMetadata, ZoomMode};

#[derive(Debug, Clone)]
pub enum EditOperation {
    Crop(Point, Point),
    RotateClockwise,
    RotateCounterclockwise,
    FlipHorizontal,
    FlipVertical,
    Resize {
        factor: f32,
        filter: usize,
    },
    RemoveBackground {
        seed: (u32, u32),
        tolerance: f32,
        grow: u32,
        feather: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Browser,
    Viewer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrowserLayout {
    Grid,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrawerMode {
    About,
    Metadata,
    Shortcuts,
    Transform,
    Settings,
}

#[derive(Debug, Clone)]
pub(crate) enum PendingAction {
    BackToBrowser,
    OpenImage { index: usize, path: PathBuf },
    OpenFileDialog,
    Exit,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDelete {
    pub path: PathBuf,
    pub permanent: bool,
    pub is_dir: bool,
}

pub struct BrowserState {
    pub current_dir: PathBuf,
    pub items: Vec<BrowserItem>,
    pub focused_index: usize,
    pub layout: BrowserLayout,
    pub nav_model: cosmic::widget::nav_bar::Model,
    pub nav_entities: HashMap<PathBuf, cosmic::widget::nav_bar::Id>,
    pub directory_generation: usize,
}

pub struct ViewerState {
    pub images: Vec<PathBuf>,
    pub index: usize,
    pub cache: HashMap<PathBuf, CachedImage>,
    pub original_image: Option<DynamicImage>,
    pub image_handle: Option<Handle>,
    pub image_width: f32,
    pub image_height: f32,
    pub zoom_factor: f32,
    pub zoom_mode: ZoomMode,
    pub opened_file_path: Option<PathBuf>,
    pub metadata: Option<ImageMetadata>,
}

pub struct EditorState {
    pub cropping: bool,
    pub crop_start: Option<Point>,
    pub crop_end: Option<Point>,
    pub scale_percent: f32,
    pub scale_filter: usize,
    pub scale_preview_active: bool,
    pub selecting_background: bool,
    pub background_seed: Option<(u32, u32)>,
    pub background_overlay: Option<Handle>,
    pub background_selected_pixels: usize,
    pub background_tolerance: f32,
    pub background_grow: u32,
    pub background_feather: u32,
    pub dirty: bool,
    pub history: Vec<EditOperation>,
    pub history_revision: u64,
}

pub struct SettingsState {
    pub show_hidden: bool,
}

pub struct UiState {
    pub view_mode: ViewMode,
    pub window_width: f32,
    pub window_height: f32,
    pub is_fullscreen: bool,
    pub window_id: Option<cosmic::iced::window::Id>,
    pub viewer_controls_hovered: bool,
    pub shortcuts_query: String,
    pub shortcuts_filter: usize,
    pub error_message: Option<String>,
    pub notification_message: Option<String>,
    pub drawer_mode: Option<DrawerMode>,
    pub about: About,
    pub(crate) pending_action: Option<PendingAction>,
    pub(crate) pending_delete: Option<PendingDelete>,
}
