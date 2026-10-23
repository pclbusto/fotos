use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, column, container, row, settings, text, text_input};
use cosmic::Element;

use crate::message::{Message, UiMessage};

pub const CATEGORY_FILTERS: [&str; 5] = ["Todas", "Archivo", "Edición", "Vista", "Ayuda"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Category {
    File,
    Edit,
    View,
    Help,
}

impl Category {
    const ALL: [Self; 4] = [Self::File, Self::Edit, Self::View, Self::Help];

    fn label(self) -> &'static str {
        match self {
            Self::File => "Archivo",
            Self::Edit => "Edición",
            Self::View => "Vista",
            Self::Help => "Ayuda",
        }
    }
}

struct Shortcut {
    keys: &'static [&'static str],
    name: &'static str,
    description: &'static str,
    category: Category,
}

const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        keys: &["Ctrl", "O"],
        name: "Abrir imagen",
        description: "Abre una imagen desde el selector de archivos",
        category: Category::File,
    },
    Shortcut {
        keys: &["Ctrl", "S"],
        name: "Guardar una copia",
        description: "Guarda la imagen editada en un archivo nuevo",
        category: Category::File,
    },
    Shortcut {
        keys: &["Ctrl", "W"],
        name: "Cerrar",
        description: "Cierra Fotos y consulta antes si hay cambios",
        category: Category::File,
    },
    Shortcut {
        keys: &["Ctrl", "Q"],
        name: "Salir",
        description: "Cierra Fotos y consulta antes si hay cambios",
        category: Category::File,
    },
    Shortcut {
        keys: &["Ctrl", "Z"],
        name: "Deshacer",
        description: "Deshace la última transformación",
        category: Category::Edit,
    },
    Shortcut {
        keys: &["Delete"],
        name: "Enviar a la papelera",
        description: "Elimina la imagen o carpeta seleccionada",
        category: Category::Edit,
    },
    Shortcut {
        keys: &["Ctrl", "Delete"],
        name: "Eliminar permanentemente",
        description: "Elimina la selección sin pasar por la papelera",
        category: Category::Edit,
    },
    Shortcut {
        keys: &["Ctrl", "1"],
        name: "Vista de grilla",
        description: "Muestra el navegador como una grilla de miniaturas",
        category: Category::View,
    },
    Shortcut {
        keys: &["Ctrl", "2"],
        name: "Vista de lista",
        description: "Muestra el navegador como una lista detallada",
        category: Category::View,
    },
    Shortcut {
        keys: &["Ctrl", "Espacio"],
        name: "Información de la foto",
        description: "Muestra u oculta los metadatos de la imagen",
        category: Category::View,
    },
    Shortcut {
        keys: &["← / →"],
        name: "Navegar imágenes",
        description: "Abre la imagen anterior o siguiente",
        category: Category::View,
    },
    Shortcut {
        keys: &["+ / −"],
        name: "Cambiar zoom",
        description: "Acerca o aleja la imagen",
        category: Category::View,
    },
    Shortcut {
        keys: &["Ctrl", "Rueda"],
        name: "Zoom con el mouse",
        description: "Cambia el zoom desde el lienzo",
        category: Category::View,
    },
    Shortcut {
        keys: &["Arrastre izquierdo"],
        name: "Desplazar imagen",
        description: "Mueve una imagen ampliada dentro del visor",
        category: Category::View,
    },
    Shortcut {
        keys: &["Botón central"],
        name: "Herramienta de recorte",
        description: "Activa el modo de recorte",
        category: Category::Edit,
    },
    Shortcut {
        keys: &["0"],
        name: "Tamaño real",
        description: "Muestra la imagen al 100 %",
        category: Category::View,
    },
    Shortcut {
        keys: &["F11"],
        name: "Pantalla completa",
        description: "Activa o desactiva la pantalla completa",
        category: Category::View,
    },
    Shortcut {
        keys: &["Esc"],
        name: "Cerrar o cancelar",
        description: "Cierra cualquier panel, diálogo o herramienta abierta",
        category: Category::View,
    },
    Shortcut {
        keys: &["Retroceso"],
        name: "Volver",
        description: "Vuelve del visor al navegador",
        category: Category::View,
    },
    Shortcut {
        keys: &["F1"],
        name: "Acerca de Fotos",
        description: "Muestra la versión, autoría, licencia y enlaces",
        category: Category::Help,
    },
    Shortcut {
        keys: &["Ctrl", "F1"],
        name: "Atajos de teclado",
        description: "Muestra u oculta este panel",
        category: Category::Help,
    },
];

pub fn view_header<'a>(query: &'a str, filter: usize) -> Element<'a, Message> {
    row![
        text_input::search_input("Buscar un atajo…", query)
            .on_input(|query| Message::Ui(UiMessage::ShortcutsQueryChanged(query)))
            .on_clear(Message::Ui(UiMessage::ShortcutsQueryChanged(String::new())))
            .width(Length::Fill),
        widget::dropdown(&CATEGORY_FILTERS[..], Some(filter), |filter| {
            Message::Ui(UiMessage::ShortcutsFilterSelected(filter))
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn view_content<'a>(query: &str, filter: usize) -> Element<'a, Message> {
    let wanted = filter
        .checked_sub(1)
        .and_then(|index| Category::ALL.get(index))
        .copied();
    let query = query.trim().to_lowercase();
    let mut sections = column![].spacing(16);
    let mut any = false;

    for category in Category::ALL {
        if wanted.is_some_and(|wanted| wanted != category) {
            continue;
        }
        let items: Vec<&Shortcut> = SHORTCUTS
            .iter()
            .filter(|shortcut| {
                shortcut.category == category
                    && (query.is_empty()
                        || shortcut.name.to_lowercase().contains(&query)
                        || shortcut.description.to_lowercase().contains(&query)
                        || shortcut
                            .keys
                            .join("+")
                            .to_lowercase()
                            .contains(&query.replace(' ', "")))
            })
            .collect();
        if items.is_empty() {
            continue;
        }
        any = true;
        let mut section = settings::section().title(category.label());
        for shortcut in items {
            section = section.add(shortcut_row(shortcut));
        }
        sections = sections.push(section);
    }

    if !any {
        sections = sections.push(
            container(text::body("No hay atajos que coincidan"))
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding(24),
        );
    }
    sections.into()
}

fn shortcut_row<'a>(shortcut: &Shortcut) -> Element<'a, Message> {
    let mut keys = row![].spacing(4).align_y(Alignment::Center);
    for (index, key) in shortcut.keys.iter().enumerate() {
        if index > 0 {
            keys = keys.push(text::caption("+"));
        }
        keys = keys.push(keycap(key));
    }

    settings::item_row(vec![
        column![
            text::body(shortcut.name),
            text::caption(shortcut.description)
        ]
        .spacing(2)
        .width(Length::Fill)
        .into(),
        keys.into(),
    ])
    .align_y(Alignment::Center)
    .into()
}

fn keycap<'a>(label: &'static str) -> Element<'a, Message> {
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
