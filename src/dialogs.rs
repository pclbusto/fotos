use cosmic::iced::Task;
use cosmic::Application as _;

use crate::app::AppModel;
use crate::message::{BrowserMessage, Message, UiMessage};
use crate::state::{DrawerMode, PendingAction};

impl AppModel {
    pub(crate) fn update_dialogs(&mut self, message: UiMessage) -> Task<cosmic::Action<Message>> {
        match message {
            UiMessage::ToggleAbout => {
                self.clear_scale_preview();
                if self.core.window.show_context && self.ui.drawer_mode == Some(DrawerMode::About) {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::About);
                    self.core.set_show_context(true);
                }
            }
            UiMessage::ToggleMetadata => {
                self.clear_scale_preview();
                if self.core.window.show_context
                    && self.ui.drawer_mode == Some(DrawerMode::Metadata)
                {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::Metadata);
                    self.core.set_show_context(true);
                }
            }
            UiMessage::ToggleSettings => {
                self.clear_scale_preview();
                if self.core.window.show_context
                    && self.ui.drawer_mode == Some(DrawerMode::Settings)
                {
                    self.core.set_show_context(false);
                } else {
                    self.ui.drawer_mode = Some(DrawerMode::Settings);
                    self.core.set_show_context(true);
                }
            }
            UiMessage::ShowHiddenChanged(show) => {
                self.settings.show_hidden = show;
                let path = self.browser.current_dir.clone();
                return self.update(Message::Browser(BrowserMessage::FolderSelected(Ok(path))));
            }
            UiMessage::CloseDrawer => {
                self.clear_scale_preview();
                self.core.set_show_context(false);
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
