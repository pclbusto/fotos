use cosmic::iced::keyboard::key::{Named, Physical};
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::Task;
use cosmic::Application as _;

use crate::app::AppModel;
use crate::keymap::{Action, Binding};
use crate::message::{BrowserMessage, EditorMessage, Message, UiMessage, ViewerMessage};
use crate::state::{PendingAction, ShortcutCapture, ViewMode};

impl AppModel {
    pub(crate) fn handle_escape(&mut self) -> Task<cosmic::Action<Message>> {
        if let Some(capture) = self.ui.shortcut_capture.take() {
            // Escape tras la primera tecla deja sólo esa; antes de ella, cancela.
            if let Some(first) = capture.first {
                self.commit_shortcut(capture.action, vec![first]);
            }
            return Task::none();
        }
        if self.ui.pending_delete.is_some() {
            return self.update(Message::Ui(UiMessage::CancelDelete));
        }
        if self.ui.pending_rename.is_some() {
            return self.update(Message::Ui(UiMessage::CancelRename));
        }
        if self.ui.pending_action.is_some() {
            return self.update(Message::Ui(UiMessage::CancelPendingAction));
        }
        // Cancelar una herramienta nunca debe sacar al usuario del visor.
        if self.editor.cropping {
            return self.update(Message::Editor(EditorMessage::CancelCrop));
        }
        if self.core.window.show_context {
            return self.update(Message::Ui(UiMessage::CloseDrawer));
        }
        if self.ui.error_message.is_some() {
            return self.update(Message::Ui(UiMessage::ClearError));
        }
        if self.ui.notification_message.is_some() {
            return self.update(Message::Ui(UiMessage::ClearNotification));
        }
        if self.editor.selecting_background {
            self.clear_background_selection();
            return Task::none();
        }
        if self.ui.view_mode == ViewMode::Viewer {
            return self.update(Message::Browser(BrowserMessage::BackToBrowser));
        }
        Task::none()
    }

    pub(crate) fn handle_key_press(
        &mut self,
        key: Key,
        physical: Physical,
        modifiers: Modifiers,
    ) -> Task<cosmic::Action<Message>> {
        let binding = Binding::from_event(&key, &physical, modifiers);

        let shortcuts_open = self.core.window.show_context
            && self.ui.drawer_mode == Some(crate::state::DrawerMode::Shortcuts);
        if !shortcuts_open {
            self.ui.shortcut_capture = None;
        }

        if let Some(capture) = self.ui.shortcut_capture.clone() {
            let Some(binding) = binding else {
                // Modificadores sueltos: se sigue esperando la tecla.
                return Task::none();
            };
            match capture.first {
                None => {
                    self.ui.shortcut_capture = Some(ShortcutCapture {
                        action: capture.action,
                        first: Some(binding),
                    });
                }
                Some(first) => {
                    self.ui.shortcut_capture = None;
                    // Presionar dos veces la misma tecla deja sólo esa.
                    let bindings = if first == binding {
                        vec![first]
                    } else {
                        vec![first, binding]
                    };
                    self.commit_shortcut(capture.action, bindings);
                }
            }
            return Task::none();
        }

        if self.editor.cropping && matches!(&key, Key::Named(Named::Enter)) {
            let has_selection = if self.editor.multi_crop {
                !self.editor.crop_regions.is_empty()
            } else {
                self.editor.crop_start.is_some() && self.editor.crop_end.is_some()
            };
            if has_selection {
                return self.update(Message::Editor(EditorMessage::ConfirmCrop));
            }
            return Task::none();
        }

        if let Some(binding) = &binding {
            let action = self
                .settings
                .keymap
                .actions_for(binding)
                .find(|action| self.action_available(*action));
            if let Some(action) = action {
                return self.run_action(action);
            }
        }

        match self.ui.view_mode {
            ViewMode::Browser => {
                let num_items = self.browser.items.len();
                if num_items == 0 {
                    return Task::none();
                }

                let cols = self.get_grid_cols();
                match key {
                    Key::Named(Named::ArrowLeft) if self.browser.focused_index > 0 => {
                        self.browser.focused_index -= 1;
                    }
                    Key::Named(Named::ArrowRight) if self.browser.focused_index + 1 < num_items => {
                        self.browser.focused_index += 1;
                    }
                    Key::Named(Named::ArrowUp) if self.browser.focused_index >= cols => {
                        self.browser.focused_index -= cols;
                    }
                    Key::Named(Named::ArrowDown) => {
                        if self.browser.focused_index + cols < num_items {
                            self.browser.focused_index += cols;
                        } else {
                            self.browser.focused_index = num_items - 1;
                        }
                    }
                    Key::Named(Named::Enter) => {
                        return self.handle_item_clicked(self.browser.focused_index);
                    }
                    _ => {}
                }
            }
            ViewMode::Viewer => {}
        }

        Task::none()
    }
}

impl AppModel {
    fn commit_shortcut(&mut self, action: Action, bindings: Vec<Binding>) {
        let displaced = self.settings.keymap.assign(action, bindings);
        if !displaced.is_empty() {
            let names: Vec<&str> = displaced.iter().map(|action| action.name()).collect();
            self.ui.notification_message =
                Some(format!("La tecla se quitó de: {}", names.join(", ")));
        }
        if let Err(error) = self.settings.keymap.save() {
            self.ui.error_message = Some(format!("No se pudieron guardar los atajos: {error}"));
        }
    }

    fn action_available(&self, action: Action) -> bool {
        let viewer = self.ui.view_mode == ViewMode::Viewer;
        match action {
            Action::OpenFile
            | Action::Undo
            | Action::Crop
            | Action::MultiCrop
            | Action::ToggleMetadata
            | Action::ZoomIn
            | Action::ZoomOut
            | Action::ZoomActual
            | Action::Back => viewer,
            Action::SaveCopy => viewer && self.viewer.image_handle.is_some(),
            Action::PreviousImage | Action::NextImage => viewer && !self.viewer.images.is_empty(),
            Action::GridView | Action::ListView => self.ui.view_mode == ViewMode::Browser,
            Action::Quit
            | Action::Trash
            | Action::DeletePermanent
            | Action::Rename
            | Action::Fullscreen
            | Action::About
            | Action::Folders
            | Action::Shortcuts => true,
        }
    }

    pub(crate) fn run_action(&mut self, action: Action) -> Task<cosmic::Action<Message>> {
        if !self.action_available(action) {
            return Task::none();
        }
        let message = match action {
            Action::OpenFile => Message::Viewer(ViewerMessage::OpenFile),
            Action::SaveCopy => Message::Viewer(ViewerMessage::SaveCopy),
            Action::Quit => Message::Ui(UiMessage::RequestExit),
            Action::Undo => Message::Editor(EditorMessage::Undo),
            Action::Crop => Message::Editor(EditorMessage::StartCrop { multiple: false }),
            Action::MultiCrop => Message::Editor(EditorMessage::StartCrop { multiple: true }),
            Action::Trash => Message::Ui(UiMessage::DeleteSelection { permanent: false }),
            Action::DeletePermanent => Message::Ui(UiMessage::DeleteSelection { permanent: true }),
            Action::Rename => Message::Ui(UiMessage::RenameSelection),
            Action::GridView => {
                Message::Browser(BrowserMessage::SetLayout(crate::state::BrowserLayout::Grid))
            }
            Action::ListView => {
                Message::Browser(BrowserMessage::SetLayout(crate::state::BrowserLayout::List))
            }
            Action::ToggleMetadata => Message::Ui(UiMessage::ToggleMetadata),
            Action::ZoomIn => Message::Viewer(ViewerMessage::ZoomIn),
            Action::ZoomOut => Message::Viewer(ViewerMessage::ZoomOut),
            Action::ZoomActual => Message::Viewer(ViewerMessage::ZoomActual),
            Action::Fullscreen => Message::Ui(UiMessage::ToggleFullscreen),
            Action::Back => Message::Browser(BrowserMessage::BackToBrowser),
            Action::About => Message::Ui(UiMessage::ToggleAbout),
            Action::Shortcuts => Message::Ui(UiMessage::ToggleShortcuts),
            Action::Folders => Message::Ui(UiMessage::ToggleFolders),
            Action::PreviousImage | Action::NextImage => {
                let len = self.viewer.images.len();
                let index = if action == Action::NextImage {
                    (self.viewer.index + 1) % len
                } else {
                    (self.viewer.index + len - 1) % len
                };
                let path = self.viewer.images[index].clone();
                if self.editor.dirty {
                    self.ui.pending_action = Some(PendingAction::OpenImage { index, path });
                    return Task::none();
                }
                return self.set_active_viewer_image(path);
            }
        };
        self.update(message)
    }
}
