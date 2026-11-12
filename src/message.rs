use std::path::PathBuf;

use cosmic::iced::keyboard::key::Physical;
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::widget::image::Handle;
use image::DynamicImage;

use crate::browser::BrowserItem;
use crate::keymap::Action;
use crate::state::BrowserLayout;
use crate::viewer::{CachedImage, ImageMetadata, ZoomMode};

#[derive(Debug, Clone)]
pub enum Message {
    Browser(BrowserMessage),
    Viewer(ViewerMessage),
    Editor(EditorMessage),
    Ui(UiMessage),
    Surface(cosmic::surface::Action<Message>),
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
    StartScan,
    ScanFinished(
        Result<(crate::database::ImageDatabase, crate::database::ScanStats), String>,
    ),
    ToggleFolderExpanded(PathBuf),
    /// Clic derecho sobre una carpeta: se recuerda antes de abrir el menú.
    FolderContext(PathBuf),
    /// Abrir una carpeta desde el panel; desde el visor vuelve al navegador.
    OpenFolder(PathBuf),
    FolderMenu(crate::browser::folders::FolderMenuAction),
    UnignoreFolder(PathBuf),
    DatabaseSaved(Result<(), String>),
}

#[derive(Debug, Clone)]
pub enum ViewerMessage {
    OpenFile,
    FileOpened(Result<(PathBuf, DynamicImage), String>),
    ResetImage,
    ImageReset(Result<(PathBuf, DynamicImage), String>),
    ZoomIn,
    ZoomOut,
    ZoomActual,
    SetZoomMode(ZoomMode),
    SaveCopy,
    FileSaved(Result<(), String>),
    SaveOverwrite,
    ActiveImageLoaded(Result<(PathBuf, DynamicImage), String>),
    PreloadCompleted(Result<(PathBuf, CachedImage), (PathBuf, String)>),
    MetadataLoaded(PathBuf, ImageMetadata),
    PanBy { x: f32, y: f32 },
    ZoomRenderReady(crate::viewer::zoom_filter::ResampleKey, Option<Handle>),
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
    ToggleBackgroundSelection,
    BackgroundSeedPicked(cosmic::iced::Point),
    BackgroundToleranceChanged(f32),
    BackgroundGrowChanged(f32),
    BackgroundFeatherChanged(f32),
    ApplyBackgroundRemoval,
    Undo,
    HistoryRebuilt {
        path: PathBuf,
        revision: u64,
        image: Result<DynamicImage, String>,
    },
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    ToggleAbout,
    ToggleMetadata,
    ToggleShortcuts,
    ShortcutsQueryChanged(String),
    ShortcutsFilterSelected(usize),
    ToggleShortcutsEditing,
    StartShortcutCapture(Action),
    CancelShortcutCapture,
    ResetShortcuts,
    RunAction(Action),
    ToggleSettings,
    ToggleFolders,
    ToggleFullscreen,
    SetViewerControlsHovered(bool),
    ShowHiddenChanged(bool),
    ZoomFilterChanged(usize),
    CloseDrawer,
    OpenUrl(String),
    ClearNotification,
    ClearError,
    DiscardChanges,
    RequestExit,
    DeleteSelection { permanent: bool },
    ConfirmDelete,
    CancelDelete,
    CancelPendingAction,
    KeyPress(Key, Physical, Modifiers),
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
