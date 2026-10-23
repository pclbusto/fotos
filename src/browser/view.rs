use cosmic::iced::widget::image::Handle;
use cosmic::iced::{Alignment, Length};
use cosmic::Element;

use crate::message::Message;

pub fn folder_tile(name: String, focused: bool) -> Element<'static, Message> {
    let icon =
        cosmic::widget::container(cosmic::widget::icon::from_name("folder-symbolic").size(96))
            .width(Length::Fixed(142.0))
            .height(Length::Fixed(112.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);
    let label = cosmic::widget::container(
        cosmic::widget::text::body(name)
            .class(cosmic::theme::Text::Custom(label_style))
            .width(Length::Fixed(136.0))
            .align_x(Alignment::Center),
    )
    .width(Length::Fixed(142.0))
    .height(Length::Fixed(30.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    let content = cosmic::iced::widget::Column::new()
        .width(Length::Fixed(142.0))
        .height(Length::Fixed(142.0))
        .align_x(Alignment::Center)
        .push(icon)
        .push(label);
    tile_container(content.into(), focused, 150.0)
}

pub fn image_tile(
    name: String,
    thumbnail: &Option<Handle>,
    focused: bool,
) -> Element<'static, Message> {
    let preview: Element<'static, Message> = thumbnail.as_ref().map_or_else(
        || {
            cosmic::widget::container(cosmic::widget::indeterminate_circular())
                .width(Length::Fixed(130.0))
                .height(Length::Fixed(120.0))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        },
        |handle| {
            cosmic::widget::image(handle.clone())
                .width(Length::Fixed(130.0))
                .height(Length::Fixed(120.0))
                .content_fit(cosmic::iced::ContentFit::Cover)
                .into()
        },
    );
    let preview = tile_container(preview, focused, 140.0);
    cosmic::iced::widget::Column::new()
        .spacing(6)
        .align_x(Alignment::Center)
        .push(preview)
        .push(
            cosmic::widget::text(name)
                .class(cosmic::theme::Text::Custom(label_style))
                .width(Length::Fixed(140.0))
                .align_x(Alignment::Center),
        )
        .into()
}

fn tile_container<'a>(
    content: Element<'a, Message>,
    focused: bool,
    height: f32,
) -> Element<'a, Message> {
    cosmic::widget::container(content)
        .width(Length::Fixed(150.0))
        .height(Length::Fixed(height))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |theme| {
            if focused {
                let cosmic_theme = theme.cosmic();
                let mut style = selection_style(theme);
                style.border.radius = cosmic_theme.corner_radii.radius_m.into();
                style
            } else {
                cosmic::iced::widget::container::Style::default()
            }
        })
        .into()
}

pub fn selection_style(theme: &cosmic::theme::Theme) -> cosmic::iced::widget::container::Style {
    let cosmic_theme = theme.cosmic();
    let background = cosmic_theme.background(theme.transparent);
    cosmic::iced::widget::container::Style {
        icon_color: Some(background.component.on.into()),
        text_color: Some(background.component.on.into()),
        background: Some(background.component.selected.into()),
        border: cosmic::iced::Border {
            color: background.component.divider.into(),
            width: 1.0,
            radius: cosmic_theme.corner_radii.radius_s.into(),
        },
        ..Default::default()
    }
}

pub fn label_style(theme: &cosmic::theme::Theme) -> cosmic::iced::widget::text::Style {
    cosmic::iced::widget::text::Style {
        color: Some(theme.cosmic().on_bg_color().into()),
        ..Default::default()
    }
}

pub fn file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
