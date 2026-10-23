use cosmic::iced::keyboard::key::Named;
use cosmic::iced::keyboard::{Key, Modifiers};
use cosmic::iced::Task;
use cosmic::Application as _;

use crate::app::AppModel;
use crate::message::{BrowserMessage, EditorMessage, Message, UiMessage, ViewerMessage};
use crate::state::{PendingAction, ViewMode};

impl AppModel {
    pub(crate) fn handle_key_press(
        &mut self,
        key: Key,
        modifiers: Modifiers,
    ) -> Task<cosmic::Action<Message>> {
        if key == Key::Named(Named::Escape) {
            if self.ui.pending_delete.is_some() {
                return self.update(Message::Ui(UiMessage::CancelDelete));
            }
            if self.ui.pending_action.is_some() {
                return self.update(Message::Ui(UiMessage::CancelPendingAction));
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
            if self.editor.cropping {
                return self.update(Message::Editor(EditorMessage::ToggleCropMode));
            }
        }

        if modifiers.control() {
            match &key {
                Key::Character(c) if c == "1" && self.ui.view_mode == ViewMode::Browser => {
                    return self.update(Message::Browser(BrowserMessage::SetLayout(
                        crate::state::BrowserLayout::Grid,
                    )));
                }
                Key::Character(c) if c == "2" && self.ui.view_mode == ViewMode::Browser => {
                    return self.update(Message::Browser(BrowserMessage::SetLayout(
                        crate::state::BrowserLayout::List,
                    )));
                }
                Key::Character(c) if c.eq_ignore_ascii_case("w") || c.eq_ignore_ascii_case("q") => {
                    return self.update(Message::Ui(UiMessage::RequestExit));
                }
                Key::Character(c)
                    if c.eq_ignore_ascii_case("o") && self.ui.view_mode == ViewMode::Viewer =>
                {
                    return self.update(Message::Viewer(ViewerMessage::OpenFile));
                }
                Key::Character(c) if c.eq_ignore_ascii_case("s") => {
                    if self.ui.view_mode == ViewMode::Viewer && self.viewer.image_handle.is_some() {
                        return self.update(Message::Viewer(ViewerMessage::SaveCopy));
                    }
                }
                Key::Character(c)
                    if c.eq_ignore_ascii_case("z") && self.ui.view_mode == ViewMode::Viewer =>
                {
                    return self.update(Message::Editor(EditorMessage::Undo));
                }
                _ => {}
            }
        }

        if key == Key::Named(Named::F11) {
            return self.update(Message::Ui(UiMessage::ToggleFullscreen));
        }

        if key == Key::Named(Named::F1) {
            let message = if modifiers.control() {
                UiMessage::ToggleShortcuts
            } else {
                UiMessage::ToggleAbout
            };
            return self.update(Message::Ui(message));
        }

        if modifiers.control()
            && matches!(&key, Key::Character(character) if character == " ")
            && self.ui.view_mode == ViewMode::Viewer
        {
            return self.update(Message::Ui(UiMessage::ToggleMetadata));
        }

        if key == Key::Named(Named::Delete) {
            return self.update(Message::Ui(UiMessage::DeleteSelection {
                permanent: modifiers.control(),
            }));
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
            ViewMode::Viewer => match key {
                Key::Named(Named::ArrowRight) if !self.viewer.images.is_empty() => {
                    let index = (self.viewer.index + 1) % self.viewer.images.len();
                    let path = self.viewer.images[index].clone();
                    if self.editor.dirty {
                        self.ui.pending_action = Some(PendingAction::OpenImage { index, path });
                    } else {
                        return self.set_active_viewer_image(path);
                    }
                }
                Key::Named(Named::ArrowLeft) if !self.viewer.images.is_empty() => {
                    let index = (self.viewer.index + self.viewer.images.len() - 1)
                        % self.viewer.images.len();
                    let path = self.viewer.images[index].clone();
                    if self.editor.dirty {
                        self.ui.pending_action = Some(PendingAction::OpenImage { index, path });
                    } else {
                        return self.set_active_viewer_image(path);
                    }
                }
                Key::Named(Named::Escape) | Key::Named(Named::Backspace) => {
                    return self.update(Message::Browser(BrowserMessage::BackToBrowser));
                }
                Key::Character(ref c) => match c.as_str() {
                    "+" | "=" => return self.update(Message::Viewer(ViewerMessage::ZoomIn)),
                    "-" => return self.update(Message::Viewer(ViewerMessage::ZoomOut)),
                    "0" => return self.update(Message::Viewer(ViewerMessage::ZoomActual)),
                    _ => {}
                },
                _ => {}
            },
        }

        Task::none()
    }
}
