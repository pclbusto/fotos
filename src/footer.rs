use cosmic::iced::{Alignment, Length};
use cosmic::Element;

use crate::app::AppModel;
use crate::message::{Message, UiMessage, ViewerMessage};
use crate::state::ViewMode;
use crate::viewer::ZoomMode;

impl AppModel {
    pub(crate) fn footer_view(&self) -> Option<Element<'_, Message>> {
        let spacing = cosmic::theme::spacing();

        if let Some(ref msg) = self.ui.notification_message {
            return Some(
                cosmic::widget::container(
                    cosmic::iced::widget::row![
                        cosmic::widget::text(msg.clone()),
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        cosmic::widget::button::text("Cerrar")
                            .on_press(Message::Ui(UiMessage::ClearNotification))
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                )
                .padding(8)
                .class(cosmic::theme::Container::Card)
                .into(),
            );
        }

        if let Some(ref err) = self.ui.error_message {
            return Some(
                cosmic::widget::container(
                    cosmic::iced::widget::row![
                        cosmic::widget::text(err.clone()),
                        cosmic::iced::widget::Space::new().width(Length::Fill),
                        cosmic::widget::button::text("Cerrar")
                            .on_press(Message::Ui(UiMessage::ClearError))
                    ]
                    .align_y(Alignment::Center)
                    .spacing(8),
                )
                .padding(8)
                .class(cosmic::theme::Container::Card)
                .into(),
            );
        }

        if self.ui.view_mode == ViewMode::Viewer && self.viewer.image_handle.is_some() {
            let prev_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("media-seek-backward-symbolic").size(16),
            )
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .on_press(Message::Ui(UiMessage::KeyPress(
                cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowLeft),
                cosmic::iced::keyboard::Modifiers::default(),
            )))
            .tooltip("Anterior");

            let next_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("media-seek-forward-symbolic").size(16),
            )
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .on_press(Message::Ui(UiMessage::KeyPress(
                cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::ArrowRight),
                cosmic::iced::keyboard::Modifiers::default(),
            )))
            .tooltip("Siguiente");

            let zoom_out = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-out-symbolic").size(16),
            )
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .on_press(Message::Viewer(ViewerMessage::ZoomOut))
            .tooltip("Alejar");

            let zoom_in = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-in-symbolic").size(16),
            )
            .width(Length::Fixed(32.0))
            .height(Length::Fixed(32.0))
            .on_press(Message::Viewer(ViewerMessage::ZoomIn))
            .tooltip("Acercar");

            let zoom_fit = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-fit-best-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::SetZoomMode(
                ZoomMode::FitWindow,
            )))
            .tooltip("Ajustar a ventana");

            let zoom_width = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-fill-width-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::SetZoomMode(
                ZoomMode::FitWidth,
            )))
            .tooltip("Ajustar al ancho");

            let zoom_actual = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("zoom-original-symbolic").size(16),
            )
            .on_press(Message::Viewer(ViewerMessage::ZoomActual))
            .tooltip("Tamaño real (100%)");

            let zoom_label =
                cosmic::widget::text(format!("{}%", (self.viewer.zoom_factor * 100.0) as u32));

            let center_row = cosmic::iced::widget::Row::new()
                .spacing(spacing.space_s)
                .align_y(Alignment::Center)
                .push(prev_btn)
                .push(zoom_out)
                .push(zoom_fit)
                .push(zoom_width)
                .push(zoom_actual)
                .push(zoom_label)
                .push(zoom_in)
                .push(next_btn);

            let info = format!(
                "{}x{} px | {}",
                self.viewer.image_width as u32,
                self.viewer.image_height as u32,
                self.viewer
                    .opened_file_path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .unwrap_or("PNG")
                    .to_uppercase()
            );
            let left_col = cosmic::widget::text(info);

            let info_btn = cosmic::widget::button::icon(
                cosmic::widget::icon::from_name("dialog-information-symbolic").size(16),
            )
            .on_press(Message::Ui(UiMessage::ToggleMetadata))
            .tooltip("Información detallada");

            let visible_controls: Element<'_, Message> = cosmic::widget::container(
                cosmic::iced::widget::row![
                    left_col,
                    cosmic::iced::widget::Space::new().width(Length::Fill),
                    center_row,
                    cosmic::iced::widget::Space::new().width(Length::Fill),
                    info_btn
                ]
                .align_y(Alignment::Center)
                .spacing(16),
            )
            .padding([4, 8])
            .into();

            let footer_content: Element<'_, Message> = if self.ui.viewer_controls_hovered {
                visible_controls
            } else {
                cosmic::iced::widget::Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(40.0))
                    .into()
            };

            Some(
                cosmic::iced::widget::mouse_area(
                    cosmic::widget::container(footer_content)
                        .width(Length::Fill)
                        .height(Length::Fixed(40.0))
                        .align_y(Alignment::Center),
                )
                .on_enter(Message::Ui(UiMessage::SetViewerControlsHovered(true)))
                .on_exit(Message::Ui(UiMessage::SetViewerControlsHovered(false)))
                .into(),
            )
        } else {
            None
        }
    }
}
