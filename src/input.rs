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
        if modifiers.control() {
            match &key {
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
                Key::Named(Named::Escape) if self.editor.selecting_background => {
                    self.editor.selecting_background = false;
                }
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
