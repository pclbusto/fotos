use cosmic::app::context_drawer::ContextDrawer;
use cosmic::iced::{Alignment, Length, Task};
use cosmic::widget::about::About;
use cosmic::Application as _;
use cosmic::Element;
use image::DynamicImage;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::browser::io as browser_io;
use crate::browser::view as browser_view;
use crate::browser::BrowserItem;
use crate::editor;
use crate::message::{BrowserMessage, EditorMessage, Message, UiMessage, ViewerMessage};
use crate::preferences::Preferences;
use crate::state::{
    BrowserState, DrawerMode, EditOperation, EditorState, PendingAction, SettingsState, UiState,
    ViewMode, ViewerState,
};
use crate::viewer::io as viewer_io;
use crate::viewer::{CachedImage, ImageCanvasProgram, ZoomMode};

pub use crate::state::BrowserLayout;

const SCALE_ALGORITHMS: [&str; 5] = [
    "Vecino más cercano",
    "Bilineal",
    "Bicúbico",
    "Gaussiano",
    "Lanczos",
];

// libcosmic currently sizes a side context drawer between 344 and 480 px while
// reserving 360 px plus 8 px of padding for the main content. Keep these values
// together so a toolkit update has a single compatibility point.
const COSMIC_DRAWER_MIN_WIDTH: f32 = 344.0;
const COSMIC_DRAWER_MAX_WIDTH: f32 = 480.0;
const COSMIC_DRAWER_CONTENT_RESERVE: f32 = 368.0;
const VIEWER_HORIZONTAL_CHROME: f32 = 40.0;
const VIEWER_VERTICAL_CHROME: f32 = 180.0;
const BROWSER_GRID_ITEM_WIDTH: f32 = 158.0;

pub struct AppModel {
    pub core: cosmic::Core,
    pub browser: BrowserState,
    pub viewer: ViewerState,
    pub editor: EditorState,
    pub settings: SettingsState,
    pub ui: UiState,
}

impl AppModel {
    pub(crate) fn save_preferences(&self) {
        let _ = Preferences::save(
            self.browser.layout,
            self.settings.show_hidden,
            &self.browser.current_dir,
        );
    }

    pub(crate) fn set_edited_image(&mut self, image: DynamicImage) {
        let handle = editor::to_handle(&image);
        self.viewer.image_width = image.width() as f32;
        self.viewer.image_height = image.height() as f32;
        self.viewer.image_handle = Some(handle.clone());
        self.viewer.original_image = Some(image);
        self.editor.crop_start = None;
        self.editor.crop_end = None;
        self.editor.cropping = false;
        self.clear_background_selection();
        self.editor.scale_preview_active = false;
        self.viewer.zoom_mode = ZoomMode::FitWindow;
        self.update_zoom_factor();
        self.editor.dirty = true;
        if let Some(path) = &self.viewer.opened_file_path {
            self.viewer.cache.insert(
                path.clone(),
                CachedImage {
                    handle,
                    width: self.viewer.image_width,
                    height: self.viewer.image_height,
                },
            );
        }
    }

    pub(crate) fn apply_edit(&mut self, image: DynamicImage, operation: EditOperation) {
        self.editor.history.push(operation);
        self.editor.history_revision = self.editor.history_revision.wrapping_add(1);
        self.set_edited_image(image);
    }

    pub(crate) fn continue_pending_action(&mut self) -> Task<cosmic::Action<Message>> {
        match self.ui.pending_action.take() {
            Some(PendingAction::BackToBrowser) => {
                self.ui.view_mode = ViewMode::Browser;
                self.core.nav_bar_set_toggled(true);
                let title = self
                    .browser
                    .current_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| self.browser.current_dir.to_string_lossy().into_owned());
                self.core.set_header_title(title);
                Task::none()
            }
            Some(PendingAction::OpenImage { index, path }) => {
                self.viewer.index = index;
                self.set_active_viewer_image(path)
            }
            Some(PendingAction::OpenFileDialog) => {
                self.update(Message::Viewer(ViewerMessage::OpenFile))
            }
            Some(PendingAction::Exit) => {
                Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Close))
            }
            None => Task::none(),
        }
    }

    pub fn get_grid_cols(&self) -> usize {
        let sidebar_w = if self.core.nav_bar_active() {
            if self.core.is_condensed() {
                60.0
            } else {
                280.0
            }
        } else {
            0.0
        };
        let available_w = (self.ui.window_width - sidebar_w - VIEWER_HORIZONTAL_CHROME).max(200.0);
        let card_w = BROWSER_GRID_ITEM_WIDTH + 8.0;
        ((available_w / card_w).floor() as usize).max(2)
    }

    fn viewer_viewport_size(&self) -> (f32, f32) {
        let drawer_width = if self.core.window.show_context
            && !self.core.window.context_is_overlay
            && self.ui.view_mode == ViewMode::Viewer
        {
            (self.ui.window_width - COSMIC_DRAWER_CONTENT_RESERVE)
                .clamp(COSMIC_DRAWER_MIN_WIDTH, COSMIC_DRAWER_MAX_WIDTH)
        } else {
            0.0
        };

        (
            (self.ui.window_width - drawer_width - VIEWER_HORIZONTAL_CHROME).max(100.0),
            (self.ui.window_height - VIEWER_VERTICAL_CHROME).max(100.0),
        )
    }

    pub fn update_zoom_factor(&mut self) {
        if self.viewer.image_width <= 0.0 || self.viewer.image_height <= 0.0 {
            self.viewer.zoom_factor = 1.0;
            return;
        }

        let (viewport_w, viewport_h) = self.viewer_viewport_size();

        match self.viewer.zoom_mode {
            ZoomMode::FitWindow => {
                let zoom_x = viewport_w / self.viewer.image_width;
                let zoom_y = viewport_h / self.viewer.image_height;
                self.viewer.zoom_factor = zoom_x.min(zoom_y).min(1.0);
            }
            ZoomMode::FitWidth => {
                self.viewer.zoom_factor = (viewport_w / self.viewer.image_width).min(1.0);
            }
            ZoomMode::RealSize => {
                self.viewer.zoom_factor = 1.0;
            }
            ZoomMode::Custom(factor) => {
                self.viewer.zoom_factor = factor;
            }
        }
    }

    pub(crate) fn manage_preloads(&mut self) -> Task<cosmic::Action<Message>> {
        if self.viewer.images.is_empty() {
            return Task::none();
        }

        let len = self.viewer.images.len();
        let idx = self.viewer.index;

        let mut active_paths = HashSet::new();
        active_paths.insert(self.viewer.images[idx].clone());
        active_paths.insert(self.viewer.images[(idx + 1) % len].clone());
        active_paths.insert(self.viewer.images[(idx + len - 1) % len].clone());

        self.viewer
            .cache
            .retain(|path, _| active_paths.contains(path));

        let mut tasks = Vec::new();
        for path in &active_paths {
            if !self.viewer.cache.contains_key(path) {
                tasks.push(viewer_io::preload_task(path.clone()));
            }
        }

        Task::batch(tasks)
    }

    pub(crate) fn set_active_viewer_image(
        &mut self,
        path: PathBuf,
    ) -> Task<cosmic::Action<Message>> {
        if let Some(index) = self.viewer.images.iter().position(|image| image == &path) {
            self.viewer.index = index;
        }
        self.editor.dirty = false;
        self.viewer.opened_file_path = Some(path.clone());
        self.viewer.original_image = None;
        self.viewer.metadata = None;
        self.editor.crop_start = None;
        self.editor.crop_end = None;
        self.clear_background_selection();
        self.editor.history.clear();
        self.editor.history_revision = self.editor.history_revision.wrapping_add(1);

        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Visor de fotos".to_string());
        self.core.set_header_title(title);

        if let Some(cached) = self.viewer.cache.get(&path) {
            self.viewer.image_handle = Some(cached.handle.clone());
            self.viewer.image_width = cached.width;
            self.viewer.image_height = cached.height;
            self.update_zoom_factor();
            self.ui.error_message = None;

            let load_task = viewer_io::load_editable_task(path.clone());
            Task::batch(vec![
                self.manage_preloads(),
                load_task,
                viewer_io::metadata_task(path),
            ])
        } else {
            self.viewer.image_handle = None;
            self.viewer.image_width = 0.0;
            self.viewer.image_height = 0.0;
            self.ui.error_message = None;

            Task::batch(vec![self.manage_preloads(), viewer_io::metadata_task(path)])
        }
    }

    pub(crate) fn handle_item_clicked(&mut self, idx: usize) -> Task<cosmic::Action<Message>> {
        if let Some(item) = self.browser.items.get(idx) {
            match item {
                BrowserItem::Folder { path, .. } => {
                    let path_clone = path.clone();
                    return Task::perform(async move { path_clone }, |p| {
                        Message::Browser(BrowserMessage::FolderSelected(Ok(p))).into()
                    });
                }
                BrowserItem::Image { path, .. } => {
                    let mut viewer_images = Vec::new();
                    let mut viewer_index = 0;

                    for sibling in &self.browser.items {
                        if let BrowserItem::Image { path: sib_path, .. } = sibling {
                            if sib_path == path {
                                viewer_index = viewer_images.len();
                            }
                            viewer_images.push(sib_path.clone());
                        }
                    }

                    self.viewer.images = viewer_images;
                    self.viewer.index = viewer_index;
                    self.ui.view_mode = ViewMode::Viewer;
                    self.core.nav_bar_set_toggled(false);

                    self.viewer.zoom_mode = ZoomMode::FitWindow;
                    return self.set_active_viewer_image(path.clone());
                }
            }
        }
        Task::none()
    }

    fn view_metadata_drawer(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let mut col = cosmic::iced::widget::Column::new()
            .spacing(spacing.space_xs)
            .padding(spacing.space_s);

        if let Some(ref path) = self.viewer.opened_file_path {
            col = col.push(cosmic::widget::text("Ubicación:"));
            col = col.push(cosmic::widget::text(path.to_string_lossy().into_owned()).size(13));

            col = col.push(cosmic::widget::text("Dimensiones:"));
            col = col.push(
                cosmic::widget::text(format!(
                    "{} x {} px",
                    self.viewer.image_width as u32, self.viewer.image_height as u32
                ))
                .size(13),
            );

            if let Some(metadata) = &self.viewer.metadata {
                let size_bytes = metadata.file_size.unwrap_or(0);
                let size_str = if size_bytes > 1024 * 1024 {
                    format!("{:.2} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else if size_bytes > 0 {
                    format!("{:.2} KB", size_bytes as f64 / 1024.0)
                } else {
                    "—".to_string()
                };
                col = col.push(cosmic::widget::text("Tamaño de archivo:"));
                col = col.push(cosmic::widget::text(size_str).size(13));
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                col = col.push(cosmic::widget::text("Formato:"));
                col = col.push(cosmic::widget::text(ext.to_uppercase()).size(13));
            }

            if let Some(metadata) = &self.viewer.metadata {
                for (label, value) in &metadata.fields {
                    col = col.push(cosmic::widget::text(format!("{label}:")));
                    col = col.push(cosmic::widget::text(value.clone()).size(13));
                }
            }
        } else {
            col = col.push(cosmic::widget::text("No hay imagen abierta"));
        }

        col.into()
    }
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = Option<std::path::PathBuf>;
    type Message = Message;
    const APP_ID: &'static str = "com.guten.Fotos";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let preferences = Preferences::load();
        let mut initial_dir = preferences
            .last_directory
            .filter(|path| path.is_dir())
            .unwrap_or_else(browser_io::pictures_dir);
        let mut initial_image = None;

        if let Some(path) = flags {
            if path.is_file() {
                if let Some(parent) = path.parent() {
                    initial_dir = parent.to_path_buf();
                }
                initial_image = Some(path);
            } else if path.is_dir() {
                initial_dir = path;
            }
        }

        let mut core = core;
        // Keep context drawers beside the image. The default overlay mode can
        // cover most of a photo, especially in a narrow window.
        core.window.context_is_overlay = false;
        core.nav_bar_set_toggled(true);

        let mut nav_entities = HashMap::new();
        let mut nav_model = cosmic::widget::nav_bar::Model::builder();

        if let Some(home) = dirs::home_dir() {
            let mut home_id = None;
            nav_model = nav_model.insert(|b| {
                b.text("Carpeta personal")
                    .icon(cosmic::widget::icon::from_name("user-home-symbolic"))
                    .data(home.clone())
                    .with_id(|id| home_id = Some(id))
            });
            if let Some(id) = home_id {
                nav_entities.insert(home.clone(), id);
            }
        }

        if let Some(pictures) = dirs::picture_dir() {
            let mut pictures_id = None;
            nav_model = nav_model.insert(|b| {
                b.text("Imágenes")
                    .icon(cosmic::widget::icon::from_name("folder-pictures-symbolic"))
                    .data(pictures.clone())
                    .with_id(|id| pictures_id = Some(id))
            });
            if let Some(id) = pictures_id {
                nav_entities.insert(pictures.clone(), id);
            }
        }

        if let Some(docs) = dirs::document_dir() {
            let mut docs_id = None;
            nav_model = nav_model.insert(|b| {
                b.text("Documentos")
                    .icon(cosmic::widget::icon::from_name("folder-documents-symbolic"))
                    .data(docs.clone())
                    .with_id(|id| docs_id = Some(id))
            });
            if let Some(id) = docs_id {
                nav_entities.insert(docs.clone(), id);
            }
        }

        if let Some(downloads) = dirs::download_dir() {
            let mut downloads_id = None;
            nav_model = nav_model.insert(|b| {
                b.text("Descargas")
                    .icon(cosmic::widget::icon::from_name("folder-download-symbolic"))
                    .data(downloads.clone())
                    .with_id(|id| downloads_id = Some(id))
            });
            if let Some(id) = downloads_id {
                nav_entities.insert(downloads.clone(), id);
            }
        }

        let mut nav_model = nav_model.build();
        if let Some(&id) = nav_entities.get(&initial_dir) {
            nav_model.activate(id);
        }

        let mut app = AppModel {
            core,
            browser: BrowserState {
                current_dir: initial_dir.clone(),
                items: Vec::new(),
                focused_index: 0,
                layout: preferences.layout,
                nav_model,
                nav_entities,
                directory_generation: 0,
            },
            viewer: ViewerState {
                images: Vec::new(),
                index: 0,
                cache: HashMap::new(),
                original_image: None,
                image_handle: None,
                image_width: 0.0,
                image_height: 0.0,
                zoom_factor: 1.0,
                zoom_mode: ZoomMode::FitWindow,
                opened_file_path: None,
                metadata: None,
            },
            editor: EditorState {
                cropping: false,
                crop_start: None,
                crop_end: None,
                scale_percent: 100.0,
                scale_filter: 4,
                scale_preview_active: false,
                selecting_background: false,
                background_seed: None,
                background_overlay: None,
                background_selected_pixels: 0,
                background_tolerance: 40.0,
                background_grow: 0,
                background_feather: 0,
                dirty: false,
                history: Vec::new(),
                history_revision: 0,
            },
            settings: SettingsState {
                show_hidden: preferences.show_hidden,
            },
            ui: UiState {
                view_mode: ViewMode::Browser,
                window_width: 900.0,
                window_height: 650.0,
                is_fullscreen: false,
                window_id: None,
                viewer_controls_hovered: false,
                error_message: None,
                notification_message: None,
                drawer_mode: None,
                about: About::default()
                    .name("Fotos")
                    .icon(cosmic::widget::icon::from_name(Self::APP_ID))
                    .version(env!("CARGO_PKG_VERSION"))
                    .author("Pedro")
                    .comments("Un visor de imágenes rápido y nativo para COSMIC Desktop.")
                    .license("MPL-2.0")
                    .license_url("https://www.mozilla.org/MPL/2.0/")
                    .developers([("Pedro", "")])
                    .links([("Repositorio", "https://github.com/guten-project/fotos")]),
                pending_action: None,
                pending_delete: None,
            },
        };

        let show_hidden = app.settings.show_hidden;
        app.save_preferences();
        let startup_task = Task::perform(
            async move { browser_io::load_directory(initial_dir, show_hidden) },
            |res| cosmic::Action::App(Message::Browser(BrowserMessage::DirectoryLoaded(res))),
        );

        if initial_image.is_some() {
            app.ui.view_mode = ViewMode::Viewer;
            app.core.nav_bar_set_toggled(false);
        }

        let mut tasks = vec![startup_task];
        if let Some(image_path) = initial_image {
            app.viewer.images = vec![image_path.clone()];
            tasks.push(app.set_active_viewer_image(image_path));
        }

        (app, Task::batch(tasks))
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        cosmic::iced::event::listen_with(|event, status, _window| {
            if status == cosmic::iced::event::Status::Ignored {
                match event {
                    cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                        key,
                        modifiers,
                        ..
                    }) => Some(Message::Ui(UiMessage::KeyPress(key, modifiers))),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn on_window_resize(&mut self, id: cosmic::iced::window::Id, width: f32, height: f32) {
        self.ui.window_id = Some(id);
        self.ui.window_width = width;
        self.ui.window_height = height;
        self.update_zoom_factor();
    }

    fn nav_model(&self) -> Option<&cosmic::widget::nav_bar::Model> {
        Some(&self.browser.nav_model)
    }

    fn on_nav_select(
        &mut self,
        id: cosmic::widget::nav_bar::Id,
    ) -> Task<cosmic::Action<Self::Message>> {
        self.browser.nav_model.activate(id);
        if let Some(path) = self.browser.nav_model.data::<PathBuf>(id) {
            let path_clone = path.clone();
            return Task::perform(async move { path_clone }, |p| {
                cosmic::Action::App(Message::Browser(BrowserMessage::FolderSelected(Ok(p))))
            });
        }
        Task::none()
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        self.header_start_view()
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        self.header_end_view()
    }

    fn context_drawer(&self) -> Option<ContextDrawer<'_, Self::Message>> {
        let mode = self.ui.drawer_mode?;
        if !self.core.window.show_context {
            return None;
        }

        match mode {
            DrawerMode::About => Some(cosmic::app::context_drawer::about(
                &self.ui.about,
                |url| Message::Ui(UiMessage::OpenUrl(url.to_owned())),
                Message::Ui(UiMessage::CloseDrawer),
            )),
            DrawerMode::Metadata => {
                let content = self.view_metadata_drawer();
                Some(
                    cosmic::app::context_drawer::context_drawer(
                        content,
                        Message::Ui(UiMessage::CloseDrawer),
                    )
                    .title("Información de la Imagen"),
                )
            }
            DrawerMode::Transform => {
                let source_width = self
                    .viewer
                    .original_image
                    .as_ref()
                    .map_or(self.viewer.image_width, |image| image.width() as f32);
                let source_height = self
                    .viewer
                    .original_image
                    .as_ref()
                    .map_or(self.viewer.image_height, |image| image.height() as f32);
                let target_width =
                    ((source_width * self.editor.scale_percent / 100.0).round() as u32).max(1);
                let target_height =
                    ((source_height * self.editor.scale_percent / 100.0).round() as u32).max(1);
                let rotate_row = cosmic::iced::widget::Row::new()
                    .spacing(cosmic::theme::spacing().space_xs)
                    .push(
                        cosmic::widget::button::standard("Izquierda")
                            .leading_icon(
                                cosmic::widget::icon::from_name("object-rotate-left-symbolic")
                                    .size(16),
                            )
                            .on_press(Message::Editor(EditorMessage::RotateCounterclockwise)),
                    )
                    .push(
                        cosmic::widget::button::standard("Derecha")
                            .leading_icon(
                                cosmic::widget::icon::from_name("object-rotate-right-symbolic")
                                    .size(16),
                            )
                            .on_press(Message::Editor(EditorMessage::RotateClockwise)),
                    );
                let flip_row = cosmic::iced::widget::Row::new()
                    .spacing(cosmic::theme::spacing().space_xs)
                    .push(
                        cosmic::widget::button::standard("Horizontal")
                            .leading_icon(
                                cosmic::widget::icon::from_name("object-flip-horizontal-symbolic")
                                    .size(16),
                            )
                            .on_press(Message::Editor(EditorMessage::FlipHorizontal)),
                    )
                    .push(
                        cosmic::widget::button::standard("Vertical")
                            .leading_icon(
                                cosmic::widget::icon::from_name("object-flip-vertical-symbolic")
                                    .size(16),
                            )
                            .on_press(Message::Editor(EditorMessage::FlipVertical)),
                    );
                let has_background_selection = self.editor.background_selected_pixels > 0;
                let content = cosmic::iced::widget::Column::new()
                    .spacing(cosmic::theme::spacing().space_s)
                    .push(cosmic::widget::text::heading("Rotar"))
                    .push(rotate_row)
                    .push(cosmic::widget::text::heading("Reflejar"))
                    .push(flip_row)
                    .push(cosmic::widget::text::heading("Escalar"))
                    .push(cosmic::widget::text::body(format!(
                        "{:.0}%  ·  {} × {} px",
                        self.editor.scale_percent, target_width, target_height,
                    )))
                    .push(
                        cosmic::widget::slider(1.0..=400.0, self.editor.scale_percent, |value| {
                            Message::Editor(EditorMessage::ScalePercentChanged(value))
                        })
                        .step(1.0)
                        .width(Length::Fill),
                    )
                    .push(cosmic::widget::dropdown(
                        &SCALE_ALGORITHMS,
                        Some(self.editor.scale_filter),
                        |value| Message::Editor(EditorMessage::ScaleFilterChanged(value)),
                    ))
                    .push(
                        cosmic::widget::button::suggested("Aplicar")
                            .leading_icon(
                                cosmic::widget::icon::from_name("emblem-ok-symbolic").size(16),
                            )
                            .on_press(Message::Editor(EditorMessage::ApplyScale)),
                    )
                    .push(cosmic::widget::text::heading("Eliminar fondo"))
                    .push(cosmic::widget::text::body(
                        "Seleccioná una zona continua; el contorno detiene la selección.",
                    ))
                    .push(
                        cosmic::widget::button::standard(if self.editor.selecting_background {
                            "Hacé clic sobre el fondo…"
                        } else {
                            "Seleccionar fondo"
                        })
                        .leading_icon(
                            cosmic::widget::icon::from_name("edit-select-all-symbolic").size(16),
                        )
                        .on_press(Message::Editor(EditorMessage::ToggleBackgroundSelection)),
                    )
                    .push(cosmic::widget::text::body(format!(
                        "Tolerancia: {:.0}",
                        self.editor.background_tolerance,
                    )))
                    .push(
                        cosmic::widget::slider(
                            0.0..=441.67294,
                            self.editor.background_tolerance,
                            |value| {
                                Message::Editor(EditorMessage::BackgroundToleranceChanged(value))
                            },
                        )
                        .step(1.0)
                        .width(Length::Fill),
                    )
                    .push(cosmic::widget::text::body(format!(
                        "Agrandar selección: {} px",
                        self.editor.background_grow,
                    )))
                    .push(
                        cosmic::widget::slider(
                            0.0..=64.0,
                            self.editor.background_grow as f32,
                            |value| Message::Editor(EditorMessage::BackgroundGrowChanged(value)),
                        )
                        .step(1.0)
                        .width(Length::Fill),
                    )
                    .push(cosmic::widget::text::body(format!(
                        "Borde suave: {} px",
                        self.editor.background_feather,
                    )))
                    .push(
                        cosmic::widget::slider(
                            0.0..=64.0,
                            self.editor.background_feather as f32,
                            |value| Message::Editor(EditorMessage::BackgroundFeatherChanged(value)),
                        )
                        .step(1.0)
                        .width(Length::Fill),
                    )
                    .push(cosmic::widget::text::body(if has_background_selection {
                        format!(
                            "{} píxeles seleccionados",
                            self.editor.background_selected_pixels
                        )
                    } else {
                        "Todavía no hay una zona seleccionada".to_string()
                    }))
                    .push(
                        cosmic::widget::button::suggested("Eliminar fondo")
                            .leading_icon(
                                cosmic::widget::icon::from_name("edit-clear-symbolic").size(16),
                            )
                            .on_press_maybe(
                                has_background_selection.then_some(Message::Editor(
                                    EditorMessage::ApplyBackgroundRemoval,
                                )),
                            ),
                    );
                Some(
                    cosmic::app::context_drawer::context_drawer(
                        cosmic::widget::scrollable(content)
                            .width(Length::Fill)
                            .height(Length::Fill),
                        Message::Ui(UiMessage::CloseDrawer),
                    )
                    .title("Transformar"),
                )
            }
            DrawerMode::Settings => {
                let content =
                    cosmic::widget::settings::section().add(cosmic::widget::settings::item(
                        "Mostrar archivos ocultos",
                        cosmic::widget::toggler(self.settings.show_hidden)
                            .on_toggle(|value| Message::Ui(UiMessage::ShowHiddenChanged(value))),
                    ));
                Some(
                    cosmic::app::context_drawer::context_drawer(
                        content,
                        Message::Ui(UiMessage::CloseDrawer),
                    )
                    .title("Ajustes"),
                )
            }
        }
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        self.dialog_view()
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.editor
            .dirty
            .then_some(Message::Ui(UiMessage::RequestExit))
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        self.footer_view()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        match self.ui.view_mode {
            ViewMode::Browser => self.view_browser(),
            ViewMode::Viewer => self.view_viewer(),
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Browser(message) => self.update_browser(message),
            Message::Editor(message) => self.update_editor(message),
            Message::Ui(UiMessage::KeyPress(key, modifiers)) => {
                self.handle_key_press(key, modifiers)
            }
            Message::Ui(message) => self.update_dialogs(message),
            Message::Viewer(message) => self.update_viewer(message),
            Message::Surface(action) => {
                cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)))
            }
        }
    }
}

// Browser View Generation
impl AppModel {
    fn view_browser(&self) -> Element<'_, Message> {
        let grid_content = cosmic::iced::widget::responsive(move |size| {
            let spacing = cosmic::theme::spacing();
            let card_w = BROWSER_GRID_ITEM_WIDTH;
            let total_item_w = card_w + spacing.space_m as f32;
            let cols = ((size.width / total_item_w).floor() as usize).max(2);

            let mut grid_col = cosmic::iced::widget::Column::new()
                .spacing(spacing.space_m)
                .width(Length::Fill);

            if self.browser.items.is_empty() {
                return cosmic::widget::container(cosmic::widget::text("Esta carpeta está vacía"))
                    .width(Length::Fill)
                    .height(Length::Fixed(200.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into();
            }

            if self.browser.layout == BrowserLayout::List {
                let mut list = cosmic::iced::widget::Column::new()
                    .spacing(1)
                    .width(Length::Fill);
                let header = cosmic::iced::widget::Row::new()
                    .spacing(spacing.space_s)
                    .align_y(Alignment::Center)
                    .padding([4, 10])
                    .push(cosmic::iced::widget::Space::new().width(Length::Fixed(40.0)))
                    .push(cosmic::widget::text::caption("Nombre").width(Length::Fill))
                    .push(cosmic::widget::text::caption("Tipo").width(Length::Fixed(110.0)))
                    .push(cosmic::widget::text::caption("Tamaño").width(Length::Fixed(90.0)));
                list = list.push(header);
                for (idx, item) in self.browser.items.iter().enumerate() {
                    let (name, kind, size, leading): (
                        String,
                        String,
                        String,
                        Element<'static, Message>,
                    ) = match item {
                        BrowserItem::Folder { name, .. } => (
                            name.clone(),
                            "Carpeta".to_string(),
                            "—".to_string(),
                            cosmic::widget::container(
                                cosmic::widget::icon::from_name("folder").size(32),
                            )
                            .width(Length::Fixed(40.0))
                            .height(Length::Fixed(40.0))
                            .align_x(Alignment::Center)
                            .align_y(Alignment::Center)
                            .into(),
                        ),
                        BrowserItem::Image {
                            name,
                            path,
                            file_size,
                            thumbnail,
                        } => {
                            let preview: Element<'static, Message> = if let Some(handle) = thumbnail
                            {
                                cosmic::widget::image(handle.clone())
                                    .width(Length::Fixed(40.0))
                                    .height(Length::Fixed(40.0))
                                    .content_fit(cosmic::iced::ContentFit::Cover)
                                    .into()
                            } else {
                                cosmic::widget::container(cosmic::widget::indeterminate_circular())
                                    .width(Length::Fixed(40.0))
                                    .height(Length::Fixed(40.0))
                                    .align_x(Alignment::Center)
                                    .align_y(Alignment::Center)
                                    .into()
                            };
                            let kind = path
                                .extension()
                                .and_then(|ext| ext.to_str())
                                .unwrap_or("Imagen")
                                .to_uppercase();
                            let size = file_size
                                .map(browser_view::file_size)
                                .unwrap_or_else(|| "—".to_string());
                            (name.clone(), kind, size, preview)
                        }
                    };
                    let row = cosmic::iced::widget::Row::new()
                        .spacing(spacing.space_s)
                        .align_y(Alignment::Center)
                        .width(Length::Fill)
                        .push(leading)
                        .push(
                            cosmic::widget::text::body(name)
                                .class(cosmic::theme::Text::Custom(browser_view::label_style))
                                .width(Length::Fill),
                        )
                        .push(cosmic::widget::text::body(kind).width(Length::Fixed(110.0)))
                        .push(cosmic::widget::text::body(size).width(Length::Fixed(90.0)));
                    let is_focused = idx == self.browser.focused_index;
                    let flat_row = cosmic::widget::container(row)
                        .width(Length::Fill)
                        .padding([2, 10])
                        .style(move |theme| {
                            if is_focused {
                                let cosmic_theme = theme.cosmic();
                                let mut style = cosmic::theme::Container::primary(cosmic_theme);
                                style.border.radius = cosmic_theme.corner_radii.radius_s.into();
                                style
                            } else {
                                cosmic::iced::widget::container::Style::default()
                            }
                        });
                    list = list.push(
                        cosmic::iced::widget::mouse_area(flat_row)
                            .on_press(Message::Browser(BrowserMessage::ItemClicked(idx))),
                    );
                }
                return list.into();
            }

            for (row_idx, chunk) in self.browser.items.chunks(cols).enumerate() {
                let mut row = cosmic::iced::widget::Row::new()
                    .spacing(spacing.space_m)
                    .width(Length::Fill);
                for (col_idx, item) in chunk.iter().enumerate() {
                    let idx = row_idx * cols + col_idx;
                    let is_focused = idx == self.browser.focused_index;

                    let item_element: Element<'static, Message> = match item {
                        BrowserItem::Image {
                            name, thumbnail, ..
                        } => {
                            let content =
                                browser_view::image_tile(name.clone(), thumbnail, is_focused);
                            cosmic::widget::button::custom(content)
                                .on_press(Message::Browser(BrowserMessage::ItemClicked(idx)))
                                .padding(4)
                                .class(cosmic::theme::Button::Transparent)
                                .into()
                        }
                        BrowserItem::Folder { name, .. } => {
                            let content = browser_view::folder_tile(name.clone(), is_focused);
                            cosmic::widget::button::custom(content)
                                .on_press(Message::Browser(BrowserMessage::ItemClicked(idx)))
                                .padding(4)
                                .class(cosmic::theme::Button::Transparent)
                                .into()
                        }
                    };
                    row = row.push(item_element);
                }
                grid_col = grid_col.push(row);
            }

            grid_col.into()
        });

        let scrollable_grid = cosmic::iced::widget::scrollable(grid_content)
            .width(Length::Fill)
            .height(Length::Fill);

        let main_col = cosmic::iced::widget::Column::new()
            .push(scrollable_grid)
            .padding(16);

        main_col.into()
    }
}

// Viewer View Generation
impl AppModel {
    fn view_viewer(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let main_content: Element<_> = if let Some(image_handle) = &self.viewer.image_handle {
            let canvas_program = ImageCanvasProgram {
                image_handle: Some(image_handle.clone()),
                img_width: self.viewer.image_width,
                img_height: self.viewer.image_height,
                zoom_factor: self.viewer.zoom_factor,
                is_cropping_mode: self.editor.cropping,
                is_selecting_background: self.editor.selecting_background,
                crop_start: self.editor.crop_start,
                crop_end: self.editor.crop_end,
            };

            let (viewport_w, viewport_h) = self.viewer_viewport_size();

            let canvas_width = self.viewer.image_width * self.viewer.zoom_factor;
            let canvas_height = self.viewer.image_height * self.viewer.zoom_factor;
            let surface_width = canvas_width.max(viewport_w);
            let surface_height = canvas_height.max(viewport_h);

            // Keep the raster image and vector crop overlay in separate layers so the
            // renderer can never composite the image above the selection rectangle.
            let image_layer = cosmic::widget::container(
                cosmic::widget::image(image_handle.clone())
                    .width(Length::Fixed(canvas_width))
                    .height(Length::Fixed(canvas_height))
                    .content_fit(cosmic::iced::ContentFit::Contain),
            )
            .width(Length::Fixed(surface_width))
            .height(Length::Fixed(surface_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

            let interaction_layer = cosmic::iced::widget::Canvas::new(canvas_program)
                .width(Length::Fixed(surface_width))
                .height(Length::Fixed(surface_height));

            let mut layers = vec![Element::from(image_layer)];
            if let Some(selection) = &self.editor.background_overlay {
                let selection_layer = cosmic::widget::container(
                    cosmic::widget::image(selection.clone())
                        .width(Length::Fixed(canvas_width))
                        .height(Length::Fixed(canvas_height))
                        .content_fit(cosmic::iced::ContentFit::Contain),
                )
                .width(Length::Fixed(surface_width))
                .height(Length::Fixed(surface_height))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center);
                layers.push(Element::from(selection_layer));
            }
            layers.push(Element::from(interaction_layer));
            let image_canvas = cosmic::iced::widget::stack(layers);

            let scrollable_canvas = cosmic::iced::widget::scrollable(image_canvas)
                .direction(cosmic::iced::widget::scrollable::Direction::Both {
                    vertical: Default::default(),
                    horizontal: Default::default(),
                })
                .width(Length::Fill)
                .height(Length::Fill);

            if self.editor.cropping {
                let mut crop_toolbar = cosmic::iced::widget::Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center);
                let crop_status = match (self.editor.crop_start, self.editor.crop_end) {
                    (Some(start), Some(end)) => format!(
                        "{} × {} px",
                        (end.x - start.x).abs().round() as u32,
                        (end.y - start.y).abs().round() as u32,
                    ),
                    _ => "Recorte".to_string(),
                };
                crop_toolbar = crop_toolbar.push(cosmic::widget::text::body(crop_status));
                crop_toolbar = crop_toolbar.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "window-close-symbolic",
                    ))
                    .on_press(Message::Editor(EditorMessage::ToggleCropMode))
                    .tooltip("Cancelar recorte"),
                );

                if self.editor.crop_start.is_some() && self.editor.crop_end.is_some() {
                    crop_toolbar = crop_toolbar.push(
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "emblem-ok-symbolic",
                        ))
                        .on_press(Message::Editor(EditorMessage::ConfirmCrop))
                        .tooltip("Aplicar recorte"),
                    );
                }

                let crop_bar_container = cosmic::widget::container(crop_toolbar)
                    .padding(8)
                    .class(cosmic::theme::Container::Card);

                cosmic::iced::widget::Column::new()
                    .spacing(8)
                    .align_x(Alignment::Center)
                    .push(crop_bar_container)
                    .push(scrollable_canvas)
                    .into()
            } else {
                scrollable_canvas.into()
            }
        } else {
            let content_col = cosmic::iced::widget::Column::new()
                .spacing(spacing.space_m)
                .align_x(Alignment::Center)
                .push(cosmic::widget::indeterminate_circular())
                .push(cosmic::widget::text("Cargando Imagen..."));

            cosmic::widget::container(content_col)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        };

        main_content
    }
}
