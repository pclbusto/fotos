use cosmic::iced::Task;

use crate::app::AppModel;
use crate::editor::{self, ResizeAlgorithm};
use crate::message::{EditorMessage, Message};
use crate::state::DrawerMode;

impl AppModel {
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
                        self.set_edited_image(cropped);
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
                    self.core.set_show_context(false);
                } else {
                    self.editor.scale_percent = 100.0;
                    self.ui.drawer_mode = Some(DrawerMode::Transform);
                    self.core.set_show_context(true);
                }
            }
            EditorMessage::RotateClockwise => {
                if let Some(image) = self.viewer.original_image.take() {
                    self.set_edited_image(editor::rotate_clockwise(image));
                }
            }
            EditorMessage::RotateCounterclockwise => {
                if let Some(image) = self.viewer.original_image.take() {
                    self.set_edited_image(editor::rotate_counterclockwise(image));
                }
            }
            EditorMessage::FlipHorizontal => {
                if let Some(image) = self.viewer.original_image.take() {
                    self.set_edited_image(editor::flip_horizontal(image));
                }
            }
            EditorMessage::FlipVertical => {
                if let Some(image) = self.viewer.original_image.take() {
                    self.set_edited_image(editor::flip_vertical(image));
                }
            }
            EditorMessage::ScalePercentChanged(percent) => {
                self.editor.scale_percent = percent.clamp(1.0, 400.0);
                self.update_scale_preview();
            }
            EditorMessage::ScaleFilterChanged(filter) => {
                self.editor.scale_filter = filter.min(4);
                self.update_scale_preview();
            }
            EditorMessage::ApplyScale => {
                if let Some(image) = self.viewer.original_image.take() {
                    let factor = self.editor.scale_percent / 100.0;
                    let algorithm = ResizeAlgorithm::from_index(self.editor.scale_filter);
                    self.set_edited_image(editor::resize(image, factor, algorithm));
                    self.editor.scale_percent = 100.0;
                }
            }
        }
        Task::none()
    }
}
