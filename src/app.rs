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
use crate::state::{
    BrowserState, DrawerMode, EditorState, PendingAction, SettingsState, UiState, ViewMode,
    ViewerState,
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

pub struct AppModel {
    pub core: cosmic::Core,
    pub browser: BrowserState,
    pub viewer: ViewerState,
    pub editor: EditorState,
    pub settings: SettingsState,
    pub ui: UiState,
}

impl AppModel {
    pub(crate) fn set_edited_image(&mut self, image: DynamicImage) {
        let handle = editor::to_handle(&image);
        self.viewer.image_width = image.width() as f32;
        self.viewer.image_height = image.height() as f32;
        self.viewer.image_handle = Some(handle.clone());
        self.viewer.original_image = Some(image);
        self.editor.crop_start = None;
        self.editor.crop_end = None;
        self.editor.cropping = false;
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
        let available_w = (self.ui.window_width - sidebar_w - 40.0).max(200.0);
        let card_w = 160.0;
        ((available_w / card_w).floor() as usize).max(2)
    }

    pub fn update_zoom_factor(&mut self) {
        if self.viewer.image_width <= 0.0 || self.viewer.image_height <= 0.0 {
            self.viewer.zoom_factor = 1.0;
            return;
        }

        let viewport_w = (self.ui.window_width - 40.0).max(100.0);
        let viewport_h = (self.ui.window_height - 180.0).max(100.0);

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
        active_paths.insert(self.viewer.images[(idx + 2) % len].clone());
        active_paths.insert(self.viewer.images[(idx + len - 1) % len].clone());
        active_paths.insert(self.viewer.images[(idx + len - 2) % len].clone());

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

    fn set_active_viewer_image(&mut self, path: PathBuf) -> Task<cosmic::Action<Message>> {
        if let Some(index) = self.viewer.images.iter().position(|image| image == &path) {
            self.viewer.index = index;
        }
        self.editor.dirty = false;
        self.viewer.opened_file_path = Some(path.clone());
        self.viewer.original_image = None;
        self.editor.crop_start = None;
        self.editor.crop_end = None;

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
            Task::batch(vec![self.manage_preloads(), load_task])
        } else {
            self.viewer.image_handle = None;
            self.viewer.image_width = 0.0;
            self.viewer.image_height = 0.0;
            self.ui.error_message = None;

            self.manage_preloads()
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

            if let Ok(metadata) = std::fs::metadata(path) {
                let size_bytes = metadata.len();
                let size_str = if size_bytes > 1024 * 1024 {
                    format!("{:.2} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.2} KB", size_bytes as f64 / 1024.0)
                };
                col = col.push(cosmic::widget::text("Tamaño de archivo:"));
                col = col.push(cosmic::widget::text(size_str).size(13));
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                col = col.push(cosmic::widget::text("Formato:"));
                col = col.push(cosmic::widget::text(ext.to_uppercase()).size(13));
            }
        } else {
            col = col.push(cosmic::widget::text("No hay imagen abierta"));
        }

        col.into()
    }
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.guten.Fotos";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let initial_dir = browser_io::pictures_dir();
        let mut core = core;
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

        let app = AppModel {
            core,
            browser: BrowserState {
                current_dir: initial_dir.clone(),
                items: Vec::new(),
                focused_index: 0,
                layout: BrowserLayout::Grid,
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
            },
            editor: EditorState {
                cropping: false,
                crop_start: None,
                crop_end: None,
                scale_percent: 100.0,
                scale_filter: 4,
                scale_preview_active: false,
                dirty: false,
            },
            settings: SettingsState { show_hidden: false },
            ui: UiState {
                view_mode: ViewMode::Browser,
                window_width: 900.0,
                window_height: 650.0,
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
            },
        };

        let startup_task = Task::perform(
            async move { browser_io::load_directory(initial_dir, false) },
            |res| cosmic::Action::App(Message::Browser(BrowserMessage::DirectoryLoaded(res))),
        );

        (app, startup_task)
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

    fn on_window_resize(&mut self, _id: cosmic::iced::window::Id, width: f32, height: f32) {
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
        let mut elements = Vec::new();
        match self.ui.view_mode {
            ViewMode::Viewer => {
                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "go-previous-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Browser(BrowserMessage::BackToBrowser))
                    .tooltip("Volver al navegador")
                    .into(),
                );
            }
            ViewMode::Browser => {}
        }
        elements
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::new();
        match self.ui.view_mode {
            ViewMode::Browser => {
                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "view-grid-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .selected(self.browser.layout == BrowserLayout::Grid)
                    .on_press(Message::Browser(BrowserMessage::SetLayout(
                        BrowserLayout::Grid,
                    )))
                    .tooltip("Vista en grilla")
                    .into(),
                );
                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "view-list-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .selected(self.browser.layout == BrowserLayout::List)
                    .on_press(Message::Browser(BrowserMessage::SetLayout(
                        BrowserLayout::List,
                    )))
                    .tooltip("Vista en lista")
                    .into(),
                );
                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "help-about-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Ui(UiMessage::ToggleAbout))
                    .tooltip("Acerca de")
                    .into(),
                );
            }
            ViewMode::Viewer => {
                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "document-open-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Viewer(ViewerMessage::OpenFile))
                    .tooltip("Abrir Imagen")
                    .into(),
                );

                let crop_icon = if self.editor.cropping {
                    "edit-select-symbolic"
                } else {
                    "edit-cut-symbolic"
                };

                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(crop_icon))
                        .icon_size(14)
                        .width(Length::Fixed(28.0))
                        .height(Length::Fixed(28.0))
                        .padding(6)
                        .class(cosmic::theme::Button::HeaderBar)
                        .on_press(Message::Editor(EditorMessage::ToggleCropMode))
                        .tooltip("Recortar imagen")
                        .into(),
                );

                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "document-save-as-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Viewer(ViewerMessage::SaveCopy))
                    .tooltip("Guardar Copia")
                    .into(),
                );

                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "view-fullscreen-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Editor(EditorMessage::ToggleTransform))
                    .tooltip("Transformar imagen")
                    .into(),
                );

                elements.push(
                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                        "help-about-symbolic",
                    ))
                    .icon_size(14)
                    .width(Length::Fixed(28.0))
                    .height(Length::Fixed(28.0))
                    .padding(6)
                    .class(cosmic::theme::Button::HeaderBar)
                    .on_press(Message::Ui(UiMessage::ToggleAbout))
                    .tooltip("Acerca de")
                    .into(),
                );
            }
        }
        elements.push(
            cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                "preferences-system-symbolic",
            ))
            .icon_size(14)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .padding(6)
            .class(cosmic::theme::Button::HeaderBar)
            .on_press(Message::Ui(UiMessage::ToggleSettings))
            .tooltip("Ajustes")
            .into(),
        );
        elements
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
                    );
                Some(
                    cosmic::app::context_drawer::context_drawer(
                        content,
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
        self.ui.pending_action.as_ref()?;
        Some(
            cosmic::widget::dialog()
                .title("La imagen tiene cambios sin guardar")
                .body("Elegí qué hacer con los cambios antes de continuar.")
                .icon(cosmic::widget::icon::from_name("dialog-warning-symbolic").size(48))
                .primary_action(
                    cosmic::widget::button::suggested("Guardar copia")
                        .on_press(Message::Viewer(ViewerMessage::SaveCopy)),
                )
                .secondary_action(
                    cosmic::widget::button::standard("Sobrescribir")
                        .on_press(Message::Viewer(ViewerMessage::SaveOverwrite)),
                )
                .tertiary_action(
                    cosmic::widget::button::destructive("Descartar")
                        .on_press(Message::Ui(UiMessage::DiscardChanges)),
                )
                .into(),
        )
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        self.editor
            .dirty
            .then_some(Message::Ui(UiMessage::RequestExit))
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        let spacing = cosmic::theme::spacing();

        if let Some(ref msg) = self.ui.notification_message {
            return Some(
                cosmic::widget::container(
                    cosmic::iced::widget::row![
                        cosmic::widget::text(msg.clone()),
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        cosmic::widget::button::text("Cerrar")
                            .on_press(Message::Ui(UiMessage::ClearNotification))
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                )
                .padding(8)
                .class(cosmic::theme::Container::Card)
                .into(),
            );
        }

        if let Some(ref err) = self.ui.error_message {
            return Some(
                cosmic::widget::container(
                    cosmic::iced::widget::row![
                        cosmic::widget::text(err.clone()),
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        cosmic::widget::button::text("Cerrar")
                            .on_press(Message::Ui(UiMessage::ClearError))
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                )
                .padding(8)
                .class(cosmic::theme::Container::Card)
                .into(),
            );
        }

        if self.ui.view_mode == ViewMode::Viewer && self.viewer.image_handle.is_some() {
            let prev_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("media-seek-backward-symbolic").size(16),
            )
            .on_press(Message::Ui(UiMessage::KeyPress(
                cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowLeft),
                cosmic::iced::keyboard::Modifiers::default(),
            )))
            .tooltip("Anterior");

            let next_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("media-seek-forward-symbolic").size(16),
            )
            .on_press(Message::Ui(UiMessage::KeyPress(
                cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowRight),
                cosmic::iced::keyboard::Modifiers::default(),
            )))
            .tooltip("Siguiente");

            let zoom_out = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-out-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::ZoomOut))
            .tooltip("Alejar");

            let zoom_in = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-in-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::ZoomIn))
            .tooltip("Acercar");

            let zoom_fit = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-fit-best-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::SetZoomMode(
                ZoomMode::FitWindow,
            )))
            .tooltip("Ajustar a ventana");

            let zoom_width = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-fill-width-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::SetZoomMode(
                ZoomMode::FitWidth,
            )))
            .tooltip("Ajustar al ancho");

            let zoom_actual = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-original-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::ZoomActual))
            .tooltip("Tamaño real (100%)");

            let zoom_label =
                cosmic::widget::text(format!("{}%", (self.viewer.zoom_factor * 100.0) as u32));

            let center_row = cosmic::iced::widget::Row::new()
                .spacing(spacing.space_s)
                .align_y(Alignment::Center)
                .push(prev_btn)
                .push(zoom_out)
                .push(zoom_fit)
                .push(zoom_width)
                .push(zoom_actual)
                .push(zoom_label)
                .push(zoom_in)
                .push(next_btn);

            let info = format!(
                "{}x{} px | {}",
                self.viewer.image_width as u32,
                self.viewer.image_height as u32,
                self.viewer
                    .opened_file_path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .unwrap_or("PNG")
                    .to_uppercase()
            );
            let left_col = cosmic::widget::text(info);

            let info_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("dialog-information-symbolic").size(16),
            )
            .on_press(Message::Ui(UiMessage::ToggleMetadata))
            .tooltip("Información detallada");

            Some(
                cosmic::widget::container(
                    cosmic::iced::widget::row![
                        left_col,
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        center_row,
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        info_btn
                    ]
                    .align_y(Alignment::Center)
                    .spacing(16),
                )
                .padding(8)
                .into(),
            )
        } else {
            None
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        match self.ui.view_mode {
            ViewMode::Browser => self.view_browser(),
            ViewMode::Viewer => self.view_viewer(),
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        let message = match message {
            Message::Browser(message) => return self.update_browser(message),
            Message::Editor(message) => return self.update_editor(message),
            Message::Ui(UiMessage::KeyPress(key, modifiers)) => UiMessage::KeyPress(key, modifiers),
            Message::Ui(message) => return self.update_dialogs(message),
            Message::Viewer(message) => return self.update_viewer(message),
        };
        match message {
            UiMessage::KeyPress(key, modifiers) => {
                use cosmic::iced::keyboard::key::Named;
                use cosmic::iced::keyboard::Key;

                if modifiers.control() {
                    match &key {
                        Key::Character(c)
                            if c.eq_ignore_ascii_case("o")
                                && self.ui.view_mode == ViewMode::Viewer =>
                        {
                            return self.update(Message::Viewer(ViewerMessage::OpenFile));
                        }
                        Key::Character(c) if c.eq_ignore_ascii_case("s") => {
                            if self.ui.view_mode == ViewMode::Viewer
                                && self.viewer.image_handle.is_some()
                            {
                                let task = self.update(Message::Viewer(ViewerMessage::SaveCopy));
                                return task;
                            }
                        }
                        _ => {}
                    }
                }

                match self.ui.view_mode {
                    ViewMode::Browser => {
                        let num_items = self.browser.items.len();
                        if num_items > 0 {
                            let cols = self.get_grid_cols();
                            match key {
                                Key::Named(Named::ArrowLeft) => {
                                    if self.browser.focused_index > 0 {
                                        self.browser.focused_index -= 1;
                                    }
                                }
                                Key::Named(Named::ArrowRight) => {
                                    if self.browser.focused_index + 1 < num_items {
                                        self.browser.focused_index += 1;
                                    }
                                }
                                Key::Named(Named::ArrowUp) => {
                                    if self.browser.focused_index >= cols {
                                        self.browser.focused_index -= cols;
                                    }
                                }
                                Key::Named(Named::ArrowDown) => {
                                    if self.browser.focused_index + cols < num_items {
                                        self.browser.focused_index += cols;
                                    } else {
                                        self.browser.focused_index = num_items - 1;
                                    }
                                }
                                Key::Named(Named::Enter) => {
                                    let idx = self.browser.focused_index;
                                    return self.handle_item_clicked(idx);
                                }
                                _ => {}
                            }
                        }
                    }
                    ViewMode::Viewer => match key {
                        Key::Named(Named::ArrowRight) => {
                            if !self.viewer.images.is_empty() {
                                let index = (self.viewer.index + 1) % self.viewer.images.len();
                                let path = self.viewer.images[index].clone();
                                if self.editor.dirty {
                                    self.ui.pending_action =
                                        Some(PendingAction::OpenImage { index, path });
                                } else {
                                    return self.set_active_viewer_image(path);
                                }
                            }
                        }
                        Key::Named(Named::ArrowLeft) => {
                            if !self.viewer.images.is_empty() {
                                let index = (self.viewer.index + self.viewer.images.len() - 1)
                                    % self.viewer.images.len();
                                let path = self.viewer.images[index].clone();
                                if self.editor.dirty {
                                    self.ui.pending_action =
                                        Some(PendingAction::OpenImage { index, path });
                                } else {
                                    return self.set_active_viewer_image(path);
                                }
                            }
                        }
                        Key::Named(Named::Escape) | Key::Named(Named::Backspace) => {
                            let task = self.update(Message::Browser(BrowserMessage::BackToBrowser));
                            return task;
                        }
                        Key::Character(ref c) => match c.as_str() {
                            "+" | "=" => {
                                let task = self.update(Message::Viewer(ViewerMessage::ZoomIn));
                                return task;
                            }
                            "-" => {
                                let task = self.update(Message::Viewer(ViewerMessage::ZoomOut));
                                return task;
                            }
                            "0" => {
                                let task = self.update(Message::Viewer(ViewerMessage::ZoomActual));
                                return task;
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                }
            }
            _ => unreachable!("only keyboard messages reach application routing"),
        }
        Task::none()
    }
}

// Browser View Generation
impl AppModel {
    fn view_browser(&self) -> Element<'_, Message> {
        let grid_content = cosmic::iced::widget::responsive(move |size| {
            let spacing = cosmic::theme::spacing();
            let card_w = 140.0;
            let total_item_w = card_w + 16.0;
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
                            let size = std::fs::metadata(path)
                                .map(|metadata| browser_view::file_size(metadata.len()))
                                .unwrap_or_else(|_| "—".to_string());
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

        let main_content: Element<_> = if self.viewer.image_handle.is_some() {
            let canvas_program = ImageCanvasProgram {
                image_handle: self.viewer.image_handle.clone(),
                img_width: self.viewer.image_width,
                img_height: self.viewer.image_height,
                zoom_factor: self.viewer.zoom_factor,
                is_cropping_mode: self.editor.cropping,
                crop_start: self.editor.crop_start,
                crop_end: self.editor.crop_end,
            };

            let viewport_w = (self.ui.window_width - 40.0).max(100.0);
            let viewport_h = (self.ui.window_height - 180.0).max(100.0);

            let canvas_width = self.viewer.image_width * self.viewer.zoom_factor;
            let canvas_height = self.viewer.image_height * self.viewer.zoom_factor;
            let surface_width = canvas_width.max(viewport_w);
            let surface_height = canvas_height.max(viewport_h);

            // Keep the raster image and vector crop overlay in separate layers so the
            // renderer can never composite the image above the selection rectangle.
            let image_layer = cosmic::widget::container(
                cosmic::widget::image(self.viewer.image_handle.as_ref().unwrap().clone())
                    .width(Length::Fixed(canvas_width))
                    .height(Length::Fixed(canvas_height))
                    .content_fit(cosmic::iced::ContentFit::Contain),
            )
            .width(Length::Fixed(surface_width))
            .height(Length::Fixed(surface_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

            let crop_layer = cosmic::iced::widget::Canvas::new(canvas_program)
                .width(Length::Fixed(surface_width))
                .height(Length::Fixed(surface_height));

            let image_canvas = cosmic::iced::widget::stack(vec![
                Element::from(image_layer),
                Element::from(crop_layer),
            ]);

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
