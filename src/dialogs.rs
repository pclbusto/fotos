use cosmic::iced::Task;
use cosmic::Application as _;

use crate::app::AppModel;
use crate::message::{BrowserMessage, Message, UiMessage};
use crate::state::{DrawerMode, PendingAction, PendingDelete};

impl AppModel {
    pub(crate) fn update_dialogs(&mut self, message: UiMessage) -> Task<cosmic::Action<Message>> {
        match message {
            UiMessage::ToggleAbout => {
                self.clear_scale_preview();
                self.clear_background_selection();
                if self.core.window.show_context && self.ui.drawer_mode == Some(DrawerMode::About) {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::About);
                    self.core.set_show_context(true);
                }
            }
            UiMessage::ToggleMetadata => {
                self.clear_scale_preview();
                self.clear_background_selection();
                if self.core.window.show_context
                    && self.ui.drawer_mode == Some(DrawerMode::Metadata)
                {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::Metadata);
                    self.core.set_show_context(true);
                }
                self.update_zoom_factor();
            }
            UiMessage::ToggleSettings => {
                self.clear_scale_preview();
                self.clear_background_selection();
                if self.core.window.show_context
                    && self.ui.drawer_mode == Some(DrawerMode::Settings)
                {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::Settings);
                    self.core.set_show_context(true);
                }
            }
            UiMessage::ToggleFullscreen => {
                if let Some(id) = self.ui.window_id {
                    self.ui.is_fullscreen = !self.ui.is_fullscreen;
                    let mode = if self.ui.is_fullscreen {
                        cosmic::iced::window::Mode::Fullscreen
                    } else {
                        cosmic::iced::window::Mode::Windowed
                    };
                    let task: Task<cosmic::Action<Message>> =
                        cosmic::iced::window::set_mode(id, mode);
                    return task;
                }
            }
            UiMessage::SetViewerControlsHovered(hovered) => {
                self.ui.viewer_controls_hovered = hovered;
            }
            UiMessage::DeleteSelection { permanent } => {
                let selection = match self.ui.view_mode {
                    crate::state::ViewMode::Browser => self
                        .browser
                        .items
                        .get(self.browser.focused_index)
                        .map(|item| match item {
                            crate::browser::BrowserItem::Image { path, .. } => {
                                (path.clone(), false, false)
                            }
                            crate::browser::BrowserItem::Folder { name, path } => {
                                (path.clone(), true, name == "..")
                            }
                        }),
                    crate::state::ViewMode::Viewer => self
                        .viewer
                        .opened_file_path
                        .clone()
                        .map(|path| (path, false, false)),
                };

                if let Some((path, is_dir, is_parent_entry)) = selection {
                    if is_parent_entry {
                        self.ui.error_message =
                            Some("La entrada «..» sólo sirve para subir de carpeta".to_string());
                    } else {
                        self.ui.pending_delete = Some(PendingDelete {
                            path,
                            permanent,
                            is_dir,
                        });
                    }
                }
                return Task::none();
            }
            UiMessage::ConfirmDelete => {
                let Some(request) = self.ui.pending_delete.take() else {
                    return Task::none();
                };

                let success = if request.permanent {
                    if request.is_dir {
                        std::fs::remove_dir_all(&request.path).is_ok()
                    } else {
                        std::fs::remove_file(&request.path).is_ok()
                    }
                } else {
                    trash::delete(&request.path).is_ok()
                };

                if !success {
                    self.ui.error_message = Some(format!(
                        "No se pudo eliminar: {}",
                        request.path.to_string_lossy()
                    ));
                    return Task::none();
                }

                let mut task = Task::none();
                if self.ui.view_mode == crate::state::ViewMode::Viewer {
                    if self.viewer.images.len() > 1 {
                        let next_index = if self.viewer.index >= self.viewer.images.len() - 1 {
                            0
                        } else {
                            self.viewer.index + 1
                        };
                        let next_path = self.viewer.images[next_index].clone();

                        self.viewer.images.remove(self.viewer.index);
                        if self.viewer.index >= self.viewer.images.len() {
                            self.viewer.index = 0;
                        }

                        task = self.set_active_viewer_image(next_path);
                    } else {
                        self.ui.pending_action = Some(PendingAction::BackToBrowser);
                        task = self.continue_pending_action();
                    }
                }

                let dir = self.browser.current_dir.clone();
                return Task::batch(vec![
                    task,
                    Task::perform(async move { Ok(dir) }, |res| {
                        cosmic::Action::App(Message::Browser(BrowserMessage::FolderSelected(res)))
                    }),
                ]);
            }
            UiMessage::CancelDelete => self.ui.pending_delete = None,
            UiMessage::ShowHiddenChanged(show) => {
                self.settings.show_hidden = show;
                self.save_preferences();
                let path = self.browser.current_dir.clone();
                return self.update(Message::Browser(BrowserMessage::FolderSelected(Ok(path))));
            }
            UiMessage::CloseDrawer => {
                self.clear_scale_preview();
                self.clear_background_selection();
                self.core.set_show_context(false);
                self.update_zoom_factor();
            }
            UiMessage::OpenUrl(url) => {
                let _ = open::that(&url);
            }
            UiMessage::ClearNotification => self.ui.notification_message = None,
            UiMessage::ClearError => self.ui.error_message = None,
            UiMessage::DiscardChanges => {
                self.editor.dirty = false;
                return self.continue_pending_action();
            }
            UiMessage::RequestExit => {
                if self.editor.dirty {
                    self.ui.pending_action = Some(PendingAction::Exit);
                } else {
                    return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Close));
                }
            }
            UiMessage::KeyPress(_, _) => {
                unreachable!("keyboard input is handled by the application router")
            }
        }
        Task::none()
    }
}
