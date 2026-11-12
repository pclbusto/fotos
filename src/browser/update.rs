use cosmic::iced::Task;

use crate::app::AppModel;
use crate::browser::folders::FolderMenuAction;
use crate::browser::{io, BrowserItem};
use crate::message::{BrowserMessage, Message};
use crate::state::PendingAction;

impl AppModel {
    pub(crate) fn update_browser(
        &mut self,
        message: BrowserMessage,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            BrowserMessage::FolderSelected(Ok(path)) => {
                self.browser.current_dir = path.clone();
                self.save_preferences();
                let title = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                self.core.set_header_title(title);
                let show_hidden = self.settings.show_hidden;
                Task::perform(
                    async move { io::load_directory(path, show_hidden) },
                    |result| {
                        cosmic::Action::App(Message::Browser(BrowserMessage::DirectoryLoaded(
                            result,
                        )))
                    },
                )
            }
            BrowserMessage::FolderSelected(Err(error)) => {
                if error != "Operación cancelada" {
                    self.ui.error_message = Some(error);
                }
                Task::none()
            }
            BrowserMessage::DirectoryLoaded(Ok(items)) => {
                self.browser.items = items;
                self.browser.focused_index = 0;
                self.browser.directory_generation += 1;

                if self.ui.view_mode == crate::state::ViewMode::Viewer {
                    let mut viewer_images = Vec::new();
                    let mut viewer_index = 0;
                    if let Some(active_path) = &self.viewer.opened_file_path {
                        for sibling in &self.browser.items {
                            if let BrowserItem::Image { path: sib_path, .. } = sibling {
                                if sib_path == active_path {
                                    viewer_index = viewer_images.len();
                                }
                                viewer_images.push(sib_path.clone());
                            }
                        }
                    }
                    if !viewer_images.is_empty() {
                        self.viewer.images = viewer_images;
                        self.viewer.index = viewer_index;
                    }
                } else {
                    self.viewer.cache.clear();
                }

                let generation = self.browser.directory_generation;
                Task::batch(
                    self.browser
                        .items
                        .iter()
                        .enumerate()
                        .map(|(index, item)| {
                            io::thumbnail_task(index, item.path().to_path_buf(), generation)
                        }),
                )
            }
            BrowserMessage::DirectoryLoaded(Err(error)) => {
                self.ui.error_message = Some(error);
                Task::none()
            }
            BrowserMessage::ThumbnailLoaded {
                index,
                path,
                handle,
                generation,
            } => {
                if generation == self.browser.directory_generation {
                    if let Some(BrowserItem::Image {
                        path: item_path,
                        thumbnail,
                        ..
                    }) = self.browser.items.get_mut(index)
                    {
                        if *item_path == path {
                            *thumbnail = handle;
                        }
                    }
                }
                Task::none()
            }
            BrowserMessage::ItemClicked(index) => self.handle_item_clicked(index),
            BrowserMessage::BackToBrowser => {
                self.ui.pending_action = Some(PendingAction::BackToBrowser);
                if self.editor.dirty {
                    Task::none()
                } else {
                    self.continue_pending_action()
                }
            }
            BrowserMessage::StartScan => {
                if self.browser.scanning {
                    return Task::none();
                }
                self.browser.scanning = true;
                super::folders::scan_task(
                    self.browser.database.clone(),
                    self.settings.show_hidden,
                    self.settings.ignored_folders.clone(),
                )
            }
            BrowserMessage::ScanFinished(result) => {
                self.browser.scanning = false;
                match result {
                    Ok((mut database, stats)) => {
                        // Por si se ignoró una carpeta mientras se escaneaba.
                        if database.remove_ignored(&self.settings.ignored_folders) > 0 {
                            self.browser.database = database;
                            self.rebuild_folder_tree();
                            return super::folders::save_task(self.browser.database.clone());
                        }
                        self.browser.database = database;
                        self.rebuild_folder_tree();
                        if stats.added > 0 || stats.removed > 0 {
                            self.ui.notification_message = Some(format!(
                                "Escaneo terminado: {} imágenes ({} nuevas, {} quitadas)",
                                stats.total_indexed, stats.added, stats.removed
                            ));
                        }
                    }
                    Err(error) => self.ui.error_message = Some(error),
                }
                Task::none()
            }
            BrowserMessage::ToggleFolderExpanded(path) => {
                if !self.browser.expanded_folders.remove(&path) {
                    self.browser.expanded_folders.insert(path);
                }
                Task::none()
            }
            BrowserMessage::OpenFolder(path) => {
                if self.ui.view_mode == crate::state::ViewMode::Viewer {
                    if self.editor.dirty {
                        self.ui.error_message = Some(
                            "Guardá o descartá los cambios de la imagen antes de cambiar de carpeta"
                                .to_string(),
                        );
                        return Task::none();
                    }
                    self.ui.pending_action = Some(PendingAction::BackToBrowser);
                    let back = self.continue_pending_action();
                    let open = self.update_browser(BrowserMessage::FolderSelected(Ok(path)));
                    return Task::batch([back, open]);
                }
                self.update_browser(BrowserMessage::FolderSelected(Ok(path)))
            }
            BrowserMessage::FolderContext(path) => {
                self.browser.folder_context = Some(path);
                Task::none()
            }
            BrowserMessage::FolderMenu(action) => {
                let Some(path) = self.browser.folder_context.clone() else {
                    return Task::none();
                };
                match action {
                    FolderMenuAction::Open => self.update_browser(BrowserMessage::OpenFolder(path)),
                    FolderMenuAction::OpenInFileManager => {
                        if let Err(error) = open::that_detached(&path) {
                            self.ui.error_message =
                                Some(format!("No se pudo abrir la carpeta: {error}"));
                        }
                        Task::none()
                    }
                    FolderMenuAction::Ignore => self.ignore_folder(path),
                }
            }
            BrowserMessage::UnignoreFolder(path) => {
                if !self.settings.ignored_folders.remove(&path) {
                    return Task::none();
                }
                if let Err(error) = self.settings.ignored_folders.save() {
                    self.ui.error_message =
                        Some(format!("No se pudo guardar la lista de ignoradas: {error}"));
                }
                // Volver a escanear para recuperar sus imágenes.
                self.update_browser(BrowserMessage::StartScan)
            }
            BrowserMessage::DatabaseSaved(result) => {
                if let Err(error) = result {
                    self.ui.error_message = Some(error);
                }
                Task::none()
            }
            BrowserMessage::SetLayout(layout) => {
                self.browser.layout = layout;
                self.save_preferences();
                Task::none()
            }
        }
    }
}

impl AppModel {
    /// Agrega la carpeta a las ignoradas, quita sus imágenes de la base y
    /// la oculta del panel.
    fn ignore_folder(&mut self, path: std::path::PathBuf) -> Task<cosmic::Action<Message>> {
        if dirs::home_dir().as_deref() == Some(path.as_path()) {
            self.ui.error_message = Some("No se puede ignorar la carpeta personal".to_string());
            return Task::none();
        }
        if !self.settings.ignored_folders.insert(path.clone()) {
            return Task::none();
        }
        if let Err(error) = self.settings.ignored_folders.save() {
            self.ui.error_message =
                Some(format!("No se pudo guardar la lista de ignoradas: {error}"));
        }
        let removed = self.browser.database.remove_under(&path);
        self.browser.expanded_folders.retain(|folder| !folder.starts_with(&path));
        self.rebuild_folder_tree();
        self.ui.notification_message = Some(format!(
            "Se ignora «{}» ({removed} imágenes quitadas). Podés revertirlo en Ajustes.",
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        ));
        super::folders::save_task(self.browser.database.clone())
    }
}
