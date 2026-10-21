use cosmic::iced::Task;

use crate::app::AppModel;
use crate::codecs;
use crate::editor;
use crate::message::{Message, ViewerMessage};
use crate::state::{PendingAction, ViewMode};
use crate::viewer::{io as viewer_io, CachedImage, ZoomMode};

impl AppModel {
    pub(crate) fn update_viewer(
        &mut self,
        message: ViewerMessage,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            ViewerMessage::ZoomIn => {
                self.viewer.zoom_mode = ZoomMode::Custom((self.viewer.zoom_factor + 0.05).min(5.0));
                self.update_zoom_factor();
            }
            ViewerMessage::ZoomOut => {
                self.viewer.zoom_mode = ZoomMode::Custom((self.viewer.zoom_factor - 0.05).max(0.1));
                self.update_zoom_factor();
            }
            ViewerMessage::ZoomActual => {
                self.viewer.zoom_mode = ZoomMode::RealSize;
                self.update_zoom_factor();
            }
            ViewerMessage::SetZoomMode(mode) => {
                self.viewer.zoom_mode = mode;
                self.update_zoom_factor();
            }
            ViewerMessage::PreloadCompleted(Ok((path, cached))) => {
                self.viewer.cache.insert(path.clone(), cached.clone());
                if self.viewer.opened_file_path.as_ref() == Some(&path) {
                    self.viewer.image_handle = Some(cached.handle);
                    self.viewer.image_width = cached.width;
                    self.viewer.image_height = cached.height;
                    self.update_zoom_factor();
                    self.ui.error_message = None;
                    // The GPU handle is enough to display the image, but editing
                    // operations need the decoded pixels as well.
                    if self.viewer.original_image.is_none() {
                        return viewer_io::load_editable_task(path);
                    }
                }
            }
            ViewerMessage::PreloadCompleted(Err((path, err))) => {
                if self.viewer.opened_file_path.as_ref() == Some(&path) {
                    self.ui.error_message = Some(err);
                }
            }
            ViewerMessage::MetadataLoaded(path, metadata) => {
                if self.viewer.opened_file_path.as_ref() == Some(&path) {
                    self.viewer.metadata = Some(metadata);
                }
            }
            ViewerMessage::OpenFile => {
                if self.editor.dirty {
                    self.ui.pending_action = Some(PendingAction::OpenFileDialog);
                    return Task::none();
                }
                self.ui.error_message = None;
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("Imágenes", codecs::READ_EXTENSIONS)
                            .pick_file()
                            .await;
                        if let Some(file_handle) = file {
                            let path = file_handle.path().to_path_buf();
                            tokio::task::spawn_blocking(move || {
                                codecs::open(&path).map(|img| (path, img))
                            })
                            .await
                            .unwrap_or_else(|_| {
                                Err(image::ImageError::IoError(std::io::Error::other(
                                    "Thread spawn failed",
                                )))
                            })
                        } else {
                            Err(image::ImageError::IoError(std::io::Error::other(
                                "Operación cancelada",
                            )))
                        }
                    },
                    |res| {
                        cosmic::Action::App(Message::Viewer(ViewerMessage::FileOpened(
                            res.map_err(|e| e.to_string()),
                        )))
                    },
                );
            }
            ViewerMessage::FileOpened(Ok((path, img))) => {
                let handle = editor::to_handle(&img);
                self.viewer.opened_file_path = Some(path.clone());
                self.viewer.image_width = img.width() as f32;
                self.viewer.image_height = img.height() as f32;
                self.viewer.image_handle = Some(handle.clone());
                self.viewer.original_image = Some(img);
                self.viewer.metadata = None;
                self.clear_background_selection();
                self.editor.dirty = false;
                self.editor.history.clear();
                self.editor.history_revision = self.editor.history_revision.wrapping_add(1);
                self.ui.view_mode = ViewMode::Viewer;
                self.core.nav_bar_set_toggled(false);

                // Add sibling images
                if let Some(parent) = path.parent() {
                    let mut sibling_images = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_file() && codecs::supports(&p) {
                                sibling_images.push(p);
                            }
                        }
                    }
                    sibling_images.sort();
                    if let Some(idx) = sibling_images.iter().position(|p| p == &path) {
                        self.viewer.images = sibling_images;
                        self.viewer.index = idx;
                    }
                }

                let title = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Visor de fotos".to_string());
                self.core.set_header_title(title);

                self.viewer.zoom_mode = ZoomMode::FitWindow;
                self.update_zoom_factor();

                self.viewer.cache.insert(
                    path.clone(),
                    CachedImage {
                        handle,
                        width: self.viewer.image_width,
                        height: self.viewer.image_height,
                    },
                );

                return Task::batch(vec![self.manage_preloads(), viewer_io::metadata_task(path)]);
            }
            ViewerMessage::FileOpened(Err(err)) => {
                if err != "Operación cancelada" {
                    self.ui.error_message = Some(err);
                }
            }
            ViewerMessage::ActiveImageLoaded(Ok((path, img))) => {
                if self.viewer.opened_file_path.as_ref() == Some(&path) {
                    self.viewer.original_image = Some(img);
                }
            }
            ViewerMessage::ActiveImageLoaded(Err(err)) => {
                self.ui.error_message = Some(err);
            }
            ViewerMessage::ResetImage => {
                if let Some(ref path) = self.viewer.opened_file_path {
                    let path_clone = path.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                codecs::open(&path_clone).map(|img| (path_clone, img))
                            })
                            .await
                            .unwrap_or_else(|_| {
                                Err(image::ImageError::IoError(std::io::Error::other(
                                    "Thread spawn failed",
                                )))
                            })
                        },
                        move |res| {
                            cosmic::Action::App(Message::Viewer(ViewerMessage::ImageReset(
                                res.map_err(|e| e.to_string()),
                            )))
                        },
                    );
                }
            }
            ViewerMessage::ImageReset(Ok((path, image))) => {
                if self.viewer.opened_file_path.as_ref() == Some(&path) {
                    let handle = editor::to_handle(&image);
                    self.viewer.image_width = image.width() as f32;
                    self.viewer.image_height = image.height() as f32;
                    self.viewer.image_handle = Some(handle.clone());
                    self.viewer.original_image = Some(image);
                    self.viewer.zoom_mode = ZoomMode::FitWindow;
                    self.editor.crop_start = None;
                    self.editor.crop_end = None;
                    self.editor.cropping = false;
                    self.clear_background_selection();
                    self.editor.scale_preview_active = false;
                    self.editor.scale_percent = 100.0;
                    self.editor.dirty = false;
                    self.editor.history.clear();
                    self.editor.history_revision = self.editor.history_revision.wrapping_add(1);
                    self.viewer.cache.insert(
                        path,
                        CachedImage {
                            handle,
                            width: self.viewer.image_width,
                            height: self.viewer.image_height,
                        },
                    );
                    self.update_zoom_factor();
                }
            }
            ViewerMessage::ImageReset(Err(err)) => {
                self.ui.error_message = Some(format!("No se pudo restablecer la imagen: {err}"));
            }
            ViewerMessage::SaveCopy => {
                if let Some(ref img) = self.viewer.original_image {
                    let img_clone = img.clone();
                    return Task::perform(
                        async move {
                            let mut dialog = rfd::AsyncFileDialog::new();
                            for (name, extensions) in codecs::WRITE_FORMATS {
                                dialog = dialog.add_filter(*name, extensions);
                            }
                            let file = dialog.save_file().await;

                            if let Some(file_handle) = file {
                                let path = file_handle.path().to_path_buf();
                                tokio::task::spawn_blocking(move || codecs::save(&img_clone, &path))
                                    .await
                                    .unwrap_or_else(|_| {
                                        Err(image::ImageError::IoError(std::io::Error::other(
                                            "Thread spawn failed",
                                        )))
                                    })
                            } else {
                                Err(image::ImageError::IoError(std::io::Error::other(
                                    "Operación cancelada",
                                )))
                            }
                        },
                        |res| {
                            cosmic::Action::App(Message::Viewer(ViewerMessage::FileSaved(
                                res.map_err(|e| e.to_string()),
                            )))
                        },
                    );
                }
            }
            ViewerMessage::SaveOverwrite => {
                if let (Some(image), Some(path)) = (
                    self.viewer.original_image.clone(),
                    self.viewer.opened_file_path.clone(),
                ) {
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || codecs::save(&image, &path))
                                .await
                                .unwrap_or_else(|_| {
                                    Err(image::ImageError::IoError(std::io::Error::other(
                                        "Thread spawn failed",
                                    )))
                                })
                        },
                        |res| {
                            cosmic::Action::App(Message::Viewer(ViewerMessage::FileSaved(
                                res.map_err(|e| e.to_string()),
                            )))
                        },
                    );
                }
            }
            ViewerMessage::FileSaved(Ok(())) => {
                self.editor.dirty = false;
                self.editor.history.clear();
                self.editor.history_revision = self.editor.history_revision.wrapping_add(1);
                self.ui.notification_message = Some("Imagen guardada".to_string());
                if self.ui.pending_action.is_some() {
                    return self.continue_pending_action();
                }
            }
            ViewerMessage::FileSaved(Err(err)) => {
                if err != "Operación cancelada" {
                    self.ui.error_message = Some(format!("Error al guardar: {}", err));
                }
            }
        }
        Task::none()
    }
}
