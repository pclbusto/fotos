use std::collections::HashMap;
use std::sync::OnceLock;

use cosmic::iced::Length;
use cosmic::widget::menu::{self, KeyBind};
use cosmic::Application as _;
use cosmic::Element;

use crate::app::AppModel;
use crate::message::{BrowserMessage, EditorMessage, Message, UiMessage, ViewerMessage};
use crate::state::{BrowserLayout, ViewMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeaderAction {
    Open,
    SaveCopy,
    Crop,
    Transform,
    Undo,
    Reset,
    Fullscreen,
    Information,
    Shortcuts,
    Settings,
    About,
}

impl menu::Action for HeaderAction {
    type Message = Message;

    fn message(&self) -> Message {
        match self {
            Self::Open => Message::Viewer(ViewerMessage::OpenFile),
            Self::SaveCopy => Message::Viewer(ViewerMessage::SaveCopy),
            Self::Crop => Message::Editor(EditorMessage::ToggleCropMode),
            Self::Transform => Message::Editor(EditorMessage::ToggleTransform),
            Self::Undo => Message::Editor(EditorMessage::Undo),
            Self::Reset => Message::Viewer(ViewerMessage::ResetImage),
            Self::Fullscreen => Message::Ui(UiMessage::ToggleFullscreen),
            Self::Information => Message::Ui(UiMessage::ToggleMetadata),
            Self::Shortcuts => Message::Ui(UiMessage::ToggleShortcuts),
            Self::Settings => Message::Ui(UiMessage::ToggleSettings),
            Self::About => Message::Ui(UiMessage::ToggleAbout),
        }
    }
}

fn menu_keybinds() -> &'static HashMap<KeyBind, HeaderAction> {
    static KEYBINDS: OnceLock<HashMap<KeyBind, HeaderAction>> = OnceLock::new();
    KEYBINDS.get_or_init(HashMap::new)
}

fn menu_icon(name: &'static str) -> Option<cosmic::widget::icon::Handle> {
    Some(cosmic::widget::icon::from_name(name).into())
}

impl AppModel {
    pub(crate) fn header_start_view(&self) -> Vec<Element<'_, Message>> {
        if self.ui.view_mode != ViewMode::Viewer {
            return Vec::new();
        }

        vec![
            cosmic::widget::button::icon(cosmic::widget::icon::from_name("go-previous-symbolic"))
                .icon_size(14)
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(6)
                .class(cosmic::theme::Button::HeaderBar)
                .on_press(Message::Browser(BrowserMessage::BackToBrowser))
                .tooltip("Volver al navegador")
                .into(),
        ]
    }

    pub(crate) fn header_end_view(&self) -> Vec<Element<'_, Message>> {
        match self.ui.view_mode {
            ViewMode::Browser => self.browser_header_actions(),
            ViewMode::Viewer => vec![self.viewer_responsive_menu()],
        }
    }

    fn browser_header_actions(&self) -> Vec<Element<'_, Message>> {
        let grid =
            cosmic::widget::button::icon(cosmic::widget::icon::from_name("view-grid-symbolic"))
                .icon_size(14)
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(6)
                .class(cosmic::theme::Button::HeaderBar)
                .selected(self.browser.layout == BrowserLayout::Grid)
                .on_press(Message::Browser(BrowserMessage::SetLayout(
                    BrowserLayout::Grid,
                )))
                .tooltip("Vista en grilla")
                .into();

        let list =
            cosmic::widget::button::icon(cosmic::widget::icon::from_name("view-list-symbolic"))
                .icon_size(14)
                .width(Length::Fixed(28.0))
                .height(Length::Fixed(28.0))
                .padding(6)
                .class(cosmic::theme::Button::HeaderBar)
                .selected(self.browser.layout == BrowserLayout::List)
                .on_press(Message::Browser(BrowserMessage::SetLayout(
                    BrowserLayout::List,
                )))
                .tooltip("Vista en lista")
                .into();

        vec![
            grid,
            list,
            header_button(
                "view-fullscreen-symbolic",
                "Pantalla completa",
                UiMessage::ToggleFullscreen,
            ),
            header_button("help-about-symbolic", "Acerca de", UiMessage::ToggleAbout),
            header_button(
                "preferences-system-symbolic",
                "Ajustes",
                UiMessage::ToggleSettings,
            ),
        ]
    }

    fn viewer_responsive_menu(&self) -> Element<'_, Message> {
        let file_items = vec![
            menu::Item::Button(
                "Abrir imagen",
                menu_icon("document-open-symbolic"),
                HeaderAction::Open,
            ),
            menu::Item::Button(
                "Guardar copia",
                menu_icon("document-save-as-symbolic"),
                HeaderAction::SaveCopy,
            ),
        ];

        let mut edit_items = vec![
            menu::Item::Button(
                "Recortar",
                menu_icon("edit-cut-symbolic"),
                HeaderAction::Crop,
            ),
            menu::Item::Button(
                "Transformar",
                menu_icon("object-rotate-right-symbolic"),
                HeaderAction::Transform,
            ),
        ];
        if !self.editor.history.is_empty() {
            edit_items.push(menu::Item::Button(
                "Deshacer",
                menu_icon("edit-undo-symbolic"),
                HeaderAction::Undo,
            ));
        }
        if self.editor.dirty {
            edit_items.push(menu::Item::Button(
                "Restablecer",
                menu_icon("document-revert-symbolic"),
                HeaderAction::Reset,
            ));
        }

        let view_items = vec![
            menu::Item::Button(
                "Información",
                menu_icon("dialog-information-symbolic"),
                HeaderAction::Information,
            ),
            menu::Item::Button(
                "Pantalla completa",
                menu_icon("view-fullscreen-symbolic"),
                HeaderAction::Fullscreen,
            ),
            menu::Item::Button(
                "Ajustes",
                menu_icon("preferences-system-symbolic"),
                HeaderAction::Settings,
            ),
            menu::Item::Button(
                "Atajos",
                menu_icon("preferences-desktop-keyboard-shortcuts-symbolic"),
                HeaderAction::Shortcuts,
            ),
            menu::Item::Button(
                "Acerca de",
                menu_icon("help-about-symbolic"),
                HeaderAction::About,
            ),
        ];

        cosmic::widget::responsive_menu_bar().into_element(
            self.core(),
            menu_keybinds(),
            cosmic::widget::Id::new("viewer-responsive-menu"),
            Message::Surface,
            vec![
                ("Archivo", file_items),
                ("Editar", edit_items),
                ("Ver", view_items),
            ],
        )
    }
}

fn header_button(
    icon: &'static str,
    tooltip: &'static str,
    message: UiMessage,
) -> Element<'static, Message> {
    cosmic::widget::button::icon(cosmic::widget::icon::from_name(icon))
        .icon_size(14)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .padding(6)
        .class(cosmic::theme::Button::HeaderBar)
        .on_press(Message::Ui(message))
        .tooltip(tooltip)
        .into()
}
