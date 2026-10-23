use cosmic::iced::widget::image::Handle;
use cosmic::iced::Task;

use crate::app::AppModel;
use crate::codecs;
use crate::editor::{self, ResizeAlgorithm};
use crate::message::{EditorMessage, Message};
use crate::state::{DrawerMode, EditOperation};

impl AppModel {
    pub(crate) fn clear_background_selection(&mut self) {
        self.editor.selecting_background = false;
        self.editor.background_seed = None;
        self.editor.background_overlay = None;
        self.editor.background_selected_pixels = 0;
    }

    fn rebuild_background_selection(&mut self) {
        self.editor.background_overlay = None;
        self.editor.background_selected_pixels = 0;
        let (Some(image), Some(seed)) = (
            self.viewer.original_image.as_ref(),
            self.editor.background_seed,
        ) else {
            return;
        };

        let source = image.to_rgba8();
        if let Some(mask) = editor::selection::SelectionMask::magic_wand(
            &source,
            seed,
            self.editor.background_tolerance,
            self.editor.background_grow,
            self.editor.background_feather,
        ) {
            self.editor.background_selected_pixels = mask.selected_pixels();
            let active_theme = cosmic::theme::active();
            let accent = active_theme.cosmic().accent_color();
            let overlay = mask.overlay([
                (accent.red * 255.0).round() as u8,
                (accent.green * 255.0).round() as u8,
                (accent.blue * 255.0).round() as u8,
            ]);
            self.editor.background_overlay = Some(Handle::from_rgba(
                overlay.width(),
                overlay.height(),
                overlay.into_raw(),
            ));
        }
    }

    pub(crate) fn clear_scale_preview(&mut self) {
        if !self.editor.scale_preview_active {
            return;
        }
        if let Some(image) = self.viewer.original_image.as_ref() {
            self.viewer.image_width = image.width() as f32;
            self.viewer.image_height = image.height() as f32;
            self.update_zoom_factor();
        }
        self.editor.scale_preview_active = false;
    }

    fn update_scale_preview(&mut self) {
        let Some(image) = self.viewer.original_image.as_ref() else {
            return;
        };
        let factor = self.editor.scale_percent / 100.0;
        self.viewer.image_width = ((image.width() as f32 * factor).round()).max(1.0);
        self.viewer.image_height = ((image.height() as f32 * factor).round()).max(1.0);
        self.editor.scale_preview_active = true;
    }

    pub(crate) fn update_editor(
        &mut self,
        message: EditorMessage,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            EditorMessage::ToggleCropMode => {
                self.editor.cropping = !self.editor.cropping;
                self.clear_background_selection();
                self.editor.crop_start = None;
                self.editor.crop_end = None;
            }
            EditorMessage::CropStart(point) => {
                self.editor.crop_start = Some(point);
                self.editor.crop_end = Some(point);
            }
            EditorMessage::CropUpdate(point) | EditorMessage::CropEnd(point) => {
                self.editor.crop_end = Some(point);
            }
            EditorMessage::SetCrop(start, end) => {
                self.editor.crop_start = Some(start);
                self.editor.crop_end = Some(end);
            }
            EditorMessage::ConfirmCrop => {
                if let (Some(image), Some(first), Some(second)) = (
                    self.viewer.original_image.as_ref(),
                    self.editor.crop_start,
                    self.editor.crop_end,
                ) {
                    if let Some(cropped) = editor::crop(image, first, second) {
                        self.apply_edit(cropped, EditOperation::Crop(first, second));
                    }
                } else if self.viewer.original_image.is_none() {
                    self.ui.error_message =
                        Some("La imagen todavía se está preparando para editar".to_string());
                }
            }
            EditorMessage::ToggleTransform => {
                if self.core.window.show_context
                    && self.ui.drawer_mode == Some(DrawerMode::Transform)
                {
                    self.clear_scale_preview();
                    self.clear_background_selection();
                    self.core.set_show_context(false);
                } else {
                    self.editor.scale_percent = 100.0;
                    self.ui.drawer_mode = Some(DrawerMode::Transform);
                    self.core.set_show_context(true);
                }
            }
            EditorMessage::RotateClockwise => {
                if let Some(image) = self.viewer.original_image.as_ref() {
                    let edited = editor::rotate_clockwise(image);
                    self.apply_edit(edited, EditOperation::RotateClockwise);
                }
            }
            EditorMessage::RotateCounterclockwise => {
                if let Some(image) = self.viewer.original_image.as_ref() {
                    let edited = editor::rotate_counterclockwise(image);
                    self.apply_edit(edited, EditOperation::RotateCounterclockwise);
                }
            }
            EditorMessage::FlipHorizontal => {
                if let Some(image) = self.viewer.original_image.as_ref() {
                    let edited = editor::flip_horizontal(image);
                    self.apply_edit(edited, EditOperation::FlipHorizontal);
                }
            }
            EditorMessage::FlipVertical => {
                if let Some(image) = self.viewer.original_image.as_ref() {
                    let edited = editor::flip_vertical(image);
                    self.apply_edit(edited, EditOperation::FlipVertical);
                }
            }
            EditorMessage::ScalePercentChanged(percent) => {
                self.clear_background_selection();
                self.editor.scale_percent = percent.clamp(1.0, 400.0);
                self.update_scale_preview();
            }
            EditorMessage::ScaleFilterChanged(filter) => {
                self.clear_background_selection();
                self.editor.scale_filter = filter.min(4);
                self.update_scale_preview();
            }
            EditorMessage::ApplyScale => {
                if let Some(image) = self.viewer.original_image.as_ref() {
                    let factor = self.editor.scale_percent / 100.0;
                    let algorithm = ResizeAlgorithm::from_index(self.editor.scale_filter);
                    let edited = editor::resize(image, factor, algorithm);
                    self.apply_edit(
                        edited,
                        EditOperation::Resize {
                            factor,
                            filter: self.editor.scale_filter,
                        },
                    );
                    self.editor.scale_percent = 100.0;
                }
            }
            EditorMessage::ToggleBackgroundSelection => {
                self.clear_scale_preview();
                self.editor.selecting_background =
                    !self.editor.selecting_background && self.viewer.original_image.is_some();
                if self.editor.selecting_background {
                    self.editor.cropping = false;
                    self.editor.crop_start = None;
                    self.editor.crop_end = None;
                }
            }
            EditorMessage::BackgroundSeedPicked(point) => {
                if self.editor.selecting_background {
                    self.editor.selecting_background = false;
                    self.editor.background_seed =
                        Some((point.x.floor() as u32, point.y.floor() as u32));
                    self.rebuild_background_selection();
                }
            }
            EditorMessage::BackgroundToleranceChanged(tolerance) => {
                self.editor.background_tolerance = tolerance.clamp(0.0, 441.67294);
                self.rebuild_background_selection();
            }
            EditorMessage::BackgroundGrowChanged(grow) => {
                self.editor.background_grow = grow.round().clamp(0.0, 64.0) as u32;
                self.rebuild_background_selection();
            }
            EditorMessage::BackgroundFeatherChanged(feather) => {
                self.editor.background_feather = feather.round().clamp(0.0, 64.0) as u32;
                self.rebuild_background_selection();
            }
            EditorMessage::ApplyBackgroundRemoval => {
                if let (Some(image), Some(seed)) = (
                    self.viewer.original_image.as_ref(),
                    self.editor.background_seed,
                ) {
                    let tolerance = self.editor.background_tolerance;
                    let grow = self.editor.background_grow;
                    let feather = self.editor.background_feather;
                    let edited = editor::remove_background(image, seed, tolerance, grow, feather);
                    self.apply_edit(
                        edited,
                        EditOperation::RemoveBackground {
                            seed,
                            tolerance,
                            grow,
                            feather,
                        },
                    );
                }
            }
            EditorMessage::Undo => {
                if self.editor.history.pop().is_none() {
                    return Task::none();
                }
                self.editor.history_revision = self.editor.history_revision.wrapping_add(1);
                let revision = self.editor.history_revision;
                let operations = self.editor.history.clone();
                let Some(path) = self.viewer.opened_file_path.clone() else {
                    return Task::none();
                };
                let result_path = path.clone();
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            codecs::open(&path)
                                .map(|image| editor::replay(image, &operations))
                                .map_err(|error| error.to_string())
                        })
                        .await
                        .unwrap_or_else(|error| Err(error.to_string()))
                    },
                    move |image| {
                        cosmic::Action::App(Message::Editor(EditorMessage::HistoryRebuilt {
                            path: result_path.clone(),
                            revision,
                            image,
                        }))
                    },
                );
            }
            EditorMessage::HistoryRebuilt {
                path,
                revision,
                image,
            } => {
                if self.viewer.opened_file_path.as_ref() != Some(&path)
                    || self.editor.history_revision != revision
                {
                    return Task::none();
                }
                match image {
                    Ok(image) => {
                        self.set_edited_image(image);
                        self.editor.dirty = !self.editor.history.is_empty();
                    }
                    Err(error) => {
                        self.ui.error_message =
                            Some(format!("No se pudo reconstruir la edición: {error}"));
                    }
                }
            }
        }
        Task::none()
    }
}
