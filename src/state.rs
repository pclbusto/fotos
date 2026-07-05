use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::widget::image::Handle;
use cosmic::iced::Point;
use cosmic::widget::about::About;
use image::DynamicImage;

use crate::browser::BrowserItem;
use crate::viewer::{CachedImage, ZoomMode};

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
}

pub struct EditorState {
    pub cropping: bool,
    pub crop_start: Option<Point>,
    pub crop_end: Option<Point>,
    pub scale_percent: f32,
    pub scale_filter: usize,
    pub scale_preview_active: bool,
    pub dirty: bool,
}

pub struct SettingsState {
    pub show_hidden: bool,
}

pub struct UiState {
    pub view_mode: ViewMode,
    pub window_width: f32,
    pub window_height: f32,
    pub error_message: Option<String>,
    pub notification_message: Option<String>,
    pub drawer_mode: Option<DrawerMode>,
    pub about: About,
    pub(crate) pending_action: Option<PendingAction>,
}
