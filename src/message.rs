use std::path::PathBuf;

use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::widget::image::Handle;
use image::DynamicImage;

use crate::browser::BrowserItem;
use crate::state::BrowserLayout;
use crate::viewer::{CachedImage, ZoomMode};

#[derive(Debug, Clone)]
pub enum Message {
    Browser(BrowserMessage),
    Viewer(ViewerMessage),
    Editor(EditorMessage),
    Ui(UiMessage),
}

#[derive(Debug, Clone)]
pub enum BrowserMessage {
    FolderSelected(Result<PathBuf, String>),
    DirectoryLoaded(Result<Vec<BrowserItem>, String>),
    ThumbnailLoaded {
        index: usize,
        path: PathBuf,
        handle: Option<Handle>,
        generation: usize,
    },
    ItemClicked(usize),
    BackToBrowser,
    SetLayout(BrowserLayout),
}

#[derive(Debug, Clone)]
pub enum ViewerMessage {
    OpenFile,
    FileOpened(Result<(PathBuf, DynamicImage), String>),
    ResetImage,
    ZoomIn,
    ZoomOut,
    ZoomActual,
    SetZoomMode(ZoomMode),
    SaveCopy,
    FileSaved(Result<(), String>),
    SaveOverwrite,
    ActiveImageLoaded(Result<(PathBuf, DynamicImage), String>),
    PreloadCompleted(Result<(PathBuf, CachedImage), (PathBuf, String)>),
}

#[derive(Debug, Clone)]
pub enum EditorMessage {
    ToggleCropMode,
    CropStart(cosmic::iced::Point),
    CropUpdate(cosmic::iced::Point),
    CropEnd(cosmic::iced::Point),
    SetCrop(cosmic::iced::Point, cosmic::iced::Point),
    ConfirmCrop,
    ToggleTransform,
    RotateClockwise,
    RotateCounterclockwise,
    FlipHorizontal,
    FlipVertical,
    ScalePercentChanged(f32),
    ScaleFilterChanged(usize),
    ApplyScale,
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    ToggleAbout,
    ToggleMetadata,
    ToggleSettings,
    ShowHiddenChanged(bool),
    CloseDrawer,
    OpenUrl(String),
    ClearNotification,
    ClearError,
    DiscardChanges,
    RequestExit,
    KeyPress(Key, Modifiers),
}

impl From<BrowserMessage> for Message {
    fn from(message: BrowserMessage) -> Self {
        Self::Browser(message)
    }
}

impl From<ViewerMessage> for Message {
    fn from(message: ViewerMessage) -> Self {
        Self::Viewer(message)
    }
}

impl From<EditorMessage> for Message {
    fn from(message: EditorMessage) -> Self {
        Self::Editor(message)
    }
}

impl From<UiMessage> for Message {
    fn from(message: UiMessage) -> Self {
        Self::Ui(message)
    }
}
