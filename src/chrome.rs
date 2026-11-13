use std::collections::HashMap;
use std::sync::OnceLock;

use cosmic::iced::Length;
use cosmic::widget::menu::{self, KeyBind};
use cosmic::Application as _;
use cosmic::Element;

use crate::app::AppModel;
use crate::browser::sort::{SortKey, SortOrder};
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
    Folders,
    Shortcuts,
    Settings,
    About,
    Sort(SortOrder),
}

impl menu::Action for HeaderAction {
    type Message = Message;

    fn message(&self) -> Message {
        match self {
            Self::Open => Message::Viewer(ViewerMessage::OpenFile),
            Self::SaveCopy => Message::Viewer(ViewerMessage::SaveCopy),
            Self::Crop => Message::Editor(EditorMessage::StartCrop { multiple: false }),
            Self::Transform => Message::Editor(EditorMessage::ToggleTransform),
            Self::Undo => Message::Editor(EditorMessage::Undo),
            Self::Reset => Message::Viewer(ViewerMessage::ResetImage),
            Self::Fullscreen => Message::Ui(UiMessage::ToggleFullscreen),
            Self::Information => Message::Ui(UiMessage::ToggleMetadata),
            Self::Folders => Message::Ui(UiMessage::ToggleFolders),
            Self::Shortcuts => Message::Ui(UiMessage::ToggleShortcuts),
            Self::Settings => Message::Ui(UiMessage::ToggleSettings),
            Self::About => Message::Ui(UiMessage::ToggleAbout),
            Self::Sort(order) => Message::Browser(BrowserMessage::SetSort(*order)),
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
            self.browser_sort_menu(),
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

    fn browser_sort_menu(&self) -> Element<'_, Message> {
        let current = self.browser.sort;
        let key_item = |label: &'static str, key: SortKey| {
            menu::Item::CheckBox(
                label,
                None,
                current.key == key,
                HeaderAction::Sort(SortOrder { key, ..current }),
            )
        };
        let direction_item = |label: &'static str, descending: bool| {
            menu::Item::CheckBox(
                label,
                None,
                current.descending == descending,
                HeaderAction::Sort(SortOrder {
                    descending,
                    ..current
                }),
            )
        };
        let items = vec![
            key_item("Nombre", SortKey::Name),
            key_item("Fecha de modificación", SortKey::Modified),
            key_item("Tamaño", SortKey::Size),
            menu::Item::Divider,
            direction_item("Ascendente", false),
            direction_item("Descendente", true),
        ];

        cosmic::widget::responsive_menu_bar().into_element(
            self.core(),
            menu_keybinds(),
            cosmic::widget::Id::new("browser-sort-menu"),
            Message::Surface,
            vec![("Ordenar", items)],
        )
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
                "Carpetas",
                menu_icon("folder-symbolic"),
                HeaderAction::Folders,
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
