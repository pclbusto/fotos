use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, button, column, container, row, settings, text, text_input};
use cosmic::Element;

use crate::keymap::{Action, Binding, Category, Keymap};
use crate::message::{Message, UiMessage};
use crate::state::ShortcutCapture;

pub const CATEGORY_FILTERS: [&str; 5] = ["Todas", "Archivo", "Edición", "Vista", "Ayuda"];

/// Atajos fijos (ratón o teclas reservadas) que no se pueden reasignar.
struct FixedShortcut {
    keys: &'static [&'static str],
    name: &'static str,
    description: &'static str,
    category: Category,
}

const FIXED_SHORTCUTS: &[FixedShortcut] = &[
    FixedShortcut {
        keys: &["Ctrl", "Rueda"],
        name: "Zoom con el mouse",
        description: "Cambia el zoom desde el lienzo",
        category: Category::View,
    },
    FixedShortcut {
        keys: &["Arrastre izquierdo"],
        name: "Desplazar imagen",
        description: "Mueve una imagen ampliada dentro del visor",
        category: Category::View,
    },
    FixedShortcut {
        keys: &["Botón central"],
        name: "Herramienta de recorte",
        description: "Activa el modo de recorte",
        category: Category::Edit,
    },
    FixedShortcut {
        keys: &["Esc"],
        name: "Cerrar o cancelar",
        description: "Cierra cualquier panel, diálogo o herramienta abierta",
        category: Category::View,
    },
];

pub fn view_header<'a>(query: &'a str, filter: usize, editing: bool) -> Element<'a, Message> {
    let edit_button = if editing {
        button::suggested("Listo")
    } else {
        button::standard("Editar")
    }
    .leading_icon(widget::icon::from_name("document-edit-symbolic").size(16))
    .on_press(Message::Ui(UiMessage::ToggleShortcutsEditing));

    row![
        text_input::search_input("Buscar un atajo…", query)
            .on_input(|query| Message::Ui(UiMessage::ShortcutsQueryChanged(query)))
            .on_clear(Message::Ui(UiMessage::ShortcutsQueryChanged(String::new())))
            .width(Length::Fill),
        widget::dropdown(&CATEGORY_FILTERS[..], Some(filter), |filter| {
            Message::Ui(UiMessage::ShortcutsFilterSelected(filter))
        }),
        edit_button,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn matches_query(query: &str, name: &str, description: &str, keys: &[String]) -> bool {
    query.is_empty()
        || name.to_lowercase().contains(query)
        || description.to_lowercase().contains(query)
        || keys
            .join("+")
            .to_lowercase()
            .contains(&query.replace(' ', ""))
}

fn binding_search_text(bindings: &[Binding]) -> Vec<String> {
    bindings
        .iter()
        .map(|binding| binding.labels().join("+").replace(' ', ""))
        .collect()
}

pub fn view_content<'a>(
    query: &str,
    filter: usize,
    keymap: &Keymap,
    editing: bool,
    capture: Option<&ShortcutCapture>,
) -> Element<'a, Message> {
    let wanted = filter
        .checked_sub(1)
        .and_then(|index| Category::ALL.get(index))
        .copied();
    let query = query.trim().to_lowercase();
    let mut sections = column![].spacing(16);

    if editing {
        sections = sections.push(
            container(text::caption(
                "Elegí «Asignar» y presioná hasta dos teclas. Si presionás dos veces la misma \
                 tecla, el atajo queda sólo con esa. Esc tras la primera tecla también la deja \
                 sola; antes, cancela.",
            ))
            .padding([0, 4]),
        );
    }

    let mut any = false;
    for category in Category::ALL {
        if wanted.is_some_and(|wanted| wanted != category) {
            continue;
        }
        let mut section = settings::section().title(category.label());
        let mut has_items = false;

        for action in Action::ALL {
            if action.category() != category {
                continue;
            }
            let bindings = keymap.bindings(action);
            if !matches_query(
                &query,
                action.name(),
                action.description(),
                &binding_search_text(bindings),
            ) {
                continue;
            }
            let capturing = capture.filter(|capture| capture.action == action);
            section = section.add(action_row(action, bindings, editing, capturing));
            has_items = true;
        }

        for shortcut in FIXED_SHORTCUTS {
            if shortcut.category != category {
                continue;
            }
            let keys: Vec<String> = shortcut.keys.iter().map(|key| key.to_string()).collect();
            if !matches_query(&query, shortcut.name, shortcut.description, &keys) {
                continue;
            }
            section = section.add(fixed_row(shortcut, editing));
            has_items = true;
        }

        if has_items {
            any = true;
            sections = sections.push(section);
        }
    }

    if !any {
        sections = sections.push(
            container(text::body("No hay atajos que coincidan"))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(24),
        );
    }

    if editing {
        let reset = button::destructive("Restablecer valores predeterminados")
            .on_press_maybe(
                (!keymap.is_default()).then_some(Message::Ui(UiMessage::ResetShortcuts)),
            );
        sections = sections.push(container(reset).width(Length::Fill).center_x(Length::Fill));
    }

    sections.into()
}

fn description_column<'a>(name: &'a str, description: &'a str) -> Element<'a, Message> {
    column![text::body(name), text::caption(description)]
        .spacing(2)
        .width(Length::Fill)
        .into()
}

fn action_row<'a>(
    action: Action,
    bindings: &[Binding],
    editing: bool,
    capture: Option<&ShortcutCapture>,
) -> Element<'a, Message> {
    let mut trailing = row![].spacing(8).align_y(Alignment::Center);

    if let Some(capture) = capture {
        let prompt = match &capture.first {
            None => row![text::caption("Presioná la primera tecla…")],
            Some(first) => row![
                binding_keys(first),
                text::caption("Presioná la segunda tecla o repetí la misma"),
            ],
        };
        trailing = trailing
            .push(prompt.spacing(8).align_y(Alignment::Center))
            .push(
                button::standard("Cancelar")
                    .on_press(Message::Ui(UiMessage::CancelShortcutCapture)),
            );
    } else {
        if bindings.is_empty() {
            trailing = trailing.push(text::caption("Sin asignar"));
        }
        for (index, binding) in bindings.iter().enumerate() {
            if index > 0 {
                trailing = trailing.push(text::caption("o"));
            }
            trailing = trailing.push(binding_keys(binding));
        }
        if editing {
            trailing = trailing.push(
                button::standard("Asignar")
                    .on_press(Message::Ui(UiMessage::StartShortcutCapture(action))),
            );
        }
    }

    settings::item_row(vec![
        description_column(action.name(), action.description()),
        trailing.into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn fixed_row<'a>(shortcut: &'static FixedShortcut, editing: bool) -> Element<'a, Message> {
    let mut keys = row![].spacing(4).align_y(Alignment::Center);
    for (index, key) in shortcut.keys.iter().enumerate() {
        if index > 0 {
            keys = keys.push(text::caption("+"));
        }
        keys = keys.push(keycap(key.to_string()));
    }
    if editing {
        keys = keys.push(text::caption("Fijo"));
    }

    settings::item_row(vec![
        description_column(shortcut.name, shortcut.description),
        keys.into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn binding_keys<'a>(binding: &Binding) -> Element<'a, Message> {
    let mut keys = row![].spacing(4).align_y(Alignment::Center);
    for (index, label) in binding.labels().into_iter().enumerate() {
        if index > 0 {
            keys = keys.push(text::caption("+"));
        }
        keys = keys.push(keycap(label));
    }
    keys.into()
}

fn keycap<'a>(label: String) -> Element<'a, Message> {
    container(text::caption_heading(label))
        .padding([2, 8])
        .class(cosmic::theme::Container::custom(|theme| {
            let cosmic = theme.cosmic();
            let background = cosmic.background(theme.transparent);
            container::Style {
                background: Some(background.component.base.into()),
                border: cosmic::iced::Border {
                    color: background.component.divider.into(),
                    width: 1.0,
                    radius: cosmic.corner_radii.radius_xs.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}
