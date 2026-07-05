use cosmic::iced::Task;

use crate::app::AppModel;
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
                if let Some(&id) = self.browser.nav_entities.get(&path) {
                    self.browser.nav_model.activate(id);
                }
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
                self.viewer.cache.clear();
                let generation = self.browser.directory_generation;
                Task::batch(
                    self.browser
                        .items
                        .iter()
                        .enumerate()
                        .filter_map(|(index, item)| match item {
                            BrowserItem::Image { path, .. } => {
                                Some(io::thumbnail_task(index, path.clone(), generation))
                            }
                            BrowserItem::Folder { .. } => None,
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
            BrowserMessage::SetLayout(layout) => {
                self.browser.layout = layout;
                Task::none()
            }
        }
    }
}
