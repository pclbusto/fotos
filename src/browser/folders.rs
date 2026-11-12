//! Árbol de carpetas escaneadas y panel lateral para recorrerlas.
//!
//! Sólo aparecen las carpetas que contienen imágenes (directamente o en alguna
//! subcarpeta). Las que tienen subcarpetas muestran un botón «+» para desplegarlas.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use cosmic::iced::widget::{Column, Row, Space};
use cosmic::iced::{Alignment, Length, Task};
use cosmic::Element;

use crate::browser::view as browser_view;
use crate::database::{IgnoredFolders, ImageDatabase, ScanStats};
use crate::message::{BrowserMessage, Message};

/// Opciones del menú contextual de una carpeta. Actúan sobre la carpeta
/// guardada en `BrowserState::folder_context` (los `MenuAction` deben ser `Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderMenuAction {
    Open,
    OpenInFileManager,
    Ignore,
}

impl cosmic::widget::menu::Action for FolderMenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        Message::Browser(BrowserMessage::FolderMenu(*self))
    }
}

fn folder_menu() -> Vec<cosmic::widget::menu::Tree<Message>> {
    use cosmic::widget::menu;
    static NO_KEYS: std::sync::OnceLock<HashMap<menu::KeyBind, FolderMenuAction>> =
        std::sync::OnceLock::new();
    let icon = |name: &'static str| Some(cosmic::widget::icon::from_name(name).into());
    menu::items(
        NO_KEYS.get_or_init(HashMap::new),
        vec![
            menu::Item::Button("Abrir", icon("folder-open-symbolic"), FolderMenuAction::Open),
            menu::Item::Button(
                "Abrir en el gestor de archivos",
                icon("system-file-manager-symbolic"),
                FolderMenuAction::OpenInFileManager,
            ),
            menu::Item::Divider,
            menu::Item::Button(
                "Ignorar carpeta",
                icon("action-unavailable-symbolic"),
                FolderMenuAction::Ignore,
            ),
        ],
    )
}

/// Sangría por nivel de profundidad.
const INDENT: f32 = 16.0;
const EXPANDER_SIZE: f32 = 28.0;

#[derive(Debug, Clone, Default)]
pub struct FolderTree {
    children: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Imágenes en la carpeta y todas sus subcarpetas.
    totals: HashMap<PathBuf, usize>,
    roots: Vec<PathBuf>,
}

impl FolderTree {
    /// Construye el árbol a partir de las carpetas con imágenes. Se agregan
    /// las carpetas intermedias (hasta `home`) para poder llegar a ellas.
    pub fn build(folders: &[(PathBuf, usize)], home: Option<&Path>) -> Self {
        let mut nodes = BTreeSet::new();
        let mut totals: HashMap<PathBuf, usize> = HashMap::new();

        for (folder, count) in folders {
            if *count == 0 {
                continue;
            }
            nodes.insert(folder.clone());
            *totals.entry(folder.clone()).or_default() += count;

            let under_home = home.is_none_or(|home| folder.starts_with(home));
            if !under_home {
                continue;
            }
            let mut current = folder.as_path();
            while let Some(parent) = current.parent() {
                if home.is_some_and(|home| !parent.starts_with(home)) {
                    break;
                }
                nodes.insert(parent.to_path_buf());
                *totals.entry(parent.to_path_buf()).or_default() += count;
                current = parent;
            }
        }

        let mut children: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        let mut roots = Vec::new();
        for node in &nodes {
            match node.parent().filter(|parent| nodes.contains(*parent)) {
                Some(parent) => children
                    .entry(parent.to_path_buf())
                    .or_default()
                    .push(node.clone()),
                None => roots.push(node.clone()),
            }
        }
        let by_name = |a: &PathBuf, b: &PathBuf| {
            display_name(a)
                .to_lowercase()
                .cmp(&display_name(b).to_lowercase())
        };
        for list in children.values_mut() {
            list.sort_by(by_name);
        }
        roots.sort_by(by_name);

        Self {
            children,
            totals,
            roots,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn has_children(&self, path: &Path) -> bool {
        self.children.get(path).is_some_and(|list| !list.is_empty())
    }

    pub fn total(&self, path: &Path) -> usize {
        self.totals.get(path).copied().unwrap_or(0)
    }

    /// Filas visibles (carpeta y profundidad) según las carpetas desplegadas.
    pub fn visible_rows(&self, expanded: &HashSet<PathBuf>) -> Vec<(PathBuf, usize)> {
        let mut rows = Vec::new();
        let mut stack: Vec<(PathBuf, usize)> =
            self.roots.iter().rev().map(|root| (root.clone(), 0)).collect();
        while let Some((path, depth)) = stack.pop() {
            if expanded.contains(&path) {
                if let Some(list) = self.children.get(&path) {
                    stack.extend(list.iter().rev().map(|child| (child.clone(), depth + 1)));
                }
            }
            rows.push((path, depth));
        }
        rows
    }
}

fn display_name(path: &Path) -> String {
    if dirs::home_dir().as_deref() == Some(path) {
        return "Carpeta personal".to_string();
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Escanea toda la carpeta personal en segundo plano y guarda la base de datos.
pub fn scan_task(
    mut database: ImageDatabase,
    show_hidden: bool,
    ignored: IgnoredFolders,
) -> Task<cosmic::Action<Message>> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || -> Result<(ImageDatabase, ScanStats), String> {
                let home = dirs::home_dir()
                    .ok_or_else(|| "No se encontró la carpeta personal".to_string())?;
                let stats = database.scan_home_recursive(&home, show_hidden, &ignored)?;
                database.remove_ignored(&ignored);
                database
                    .save()
                    .map_err(|error| format!("No se pudo guardar la base de datos: {error}"))?;
                Ok((database, stats))
            })
            .await
            .map_err(|error| error.to_string())?
        },
        |result| cosmic::Action::App(Message::Browser(BrowserMessage::ScanFinished(result))),
    )
}

/// Guarda la base de datos en segundo plano.
pub fn save_task(database: ImageDatabase) -> Task<cosmic::Action<Message>> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                database
                    .save()
                    .map_err(|error| format!("No se pudo guardar la base de datos: {error}"))
            })
            .await
            .map_err(|error| error.to_string())?
        },
        |result| cosmic::Action::App(Message::Browser(BrowserMessage::DatabaseSaved(result))),
    )
}

pub fn view<'a>(
    tree: &'a FolderTree,
    expanded: &'a HashSet<PathBuf>,
    current_dir: &'a Path,
    scanning: bool,
    window_id: Option<cosmic::iced::window::Id>,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let mut column = Column::new().spacing(2).width(Length::Fill);

    let status: Element<'a, Message> = if scanning {
        Row::new()
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center)
            .push(cosmic::widget::indeterminate_circular().size(16.0))
            .push(cosmic::widget::text::caption("Escaneando la carpeta personal…"))
            .into()
    } else {
        cosmic::widget::button::standard("Volver a escanear")
            .leading_icon(cosmic::widget::icon::from_name("view-refresh-symbolic"))
            .on_press(Message::Browser(BrowserMessage::StartScan))
            .into()
    };
    column = column.push(
        cosmic::widget::container(status)
            .padding([0, 0, spacing.space_s, 0])
            .width(Length::Fill),
    );

    if tree.is_empty() {
        let text = if scanning {
            "Buscando carpetas con imágenes…"
        } else {
            "No hay carpetas escaneadas con imágenes"
        };
        return column.push(cosmic::widget::text::body(text)).into();
    }

    for (path, depth) in tree.visible_rows(expanded) {
        let is_expanded = expanded.contains(&path);
        let expander: Element<'a, Message> = if tree.has_children(&path) {
            let icon = if is_expanded {
                "list-remove-symbolic"
            } else {
                "list-add-symbolic"
            };
            cosmic::widget::button::icon(cosmic::widget::icon::from_name(icon).size(16))
                .width(Length::Fixed(EXPANDER_SIZE))
                .height(Length::Fixed(EXPANDER_SIZE))
                .on_press(Message::Browser(BrowserMessage::ToggleFolderExpanded(
                    path.clone(),
                )))
                .tooltip(if is_expanded { "Contraer" } else { "Desplegar" })
                .into()
        } else {
            Space::new().width(Length::Fixed(EXPANDER_SIZE)).into()
        };

        let label = Row::new()
            .spacing(spacing.space_xs)
            .align_y(Alignment::Center)
            .push(cosmic::widget::icon::from_name("folder-symbolic").size(16))
            .push(
                cosmic::widget::text::body(display_name(&path))
                    .class(cosmic::theme::Text::Custom(browser_view::label_style))
                    .width(Length::Fill),
            )
            .push(cosmic::widget::text::caption(tree.total(&path).to_string()));

        let is_current = path == current_dir;
        let label = cosmic::widget::container(label)
            .padding([4, 8])
            .width(Length::Fill)
            .style(move |theme| {
                if is_current {
                    browser_view::selection_style(theme)
                } else {
                    cosmic::iced::widget::container::Style::default()
                }
            });
        // El botón derecho se registra al presionar; el menú se abre al soltarlo.
        let label = cosmic::iced::widget::mouse_area(label)
            .on_press(Message::Browser(BrowserMessage::OpenFolder(path.clone())))
            .on_right_press(Message::Browser(BrowserMessage::FolderContext(path.clone())));
        let mut label = cosmic::widget::context_menu(label, Some(folder_menu()))
            .on_surface_action(Message::Surface);
        if let Some(id) = window_id {
            label = label.window_id(id);
        }

        column = column.push(
            Row::new()
                .align_y(Alignment::Center)
                .push(Space::new().width(Length::Fixed(depth as f32 * INDENT)))
                .push(expander)
                .push(label),
        );
    }

    column.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree_with_intermediate_folders() {
        let home = PathBuf::from("/h");
        let folders = vec![
            (PathBuf::from("/h/Fotos/2024"), 3),
            (PathBuf::from("/h/Fotos/2025"), 2),
            (PathBuf::from("/h/Descargas"), 1),
            (PathBuf::from("/h/Vacia"), 0),
            (PathBuf::from("/otro/lado"), 4),
        ];
        let tree = FolderTree::build(&folders, Some(&home));

        assert_eq!(tree.roots(), &[PathBuf::from("/h"), PathBuf::from("/otro/lado")]);
        assert_eq!(tree.total(Path::new("/h")), 6);
        assert_eq!(tree.total(Path::new("/h/Fotos")), 5);
        assert!(tree.has_children(Path::new("/h/Fotos")));
        assert!(!tree.has_children(Path::new("/h/Descargas")));
        assert_eq!(tree.total(Path::new("/h/Vacia")), 0);

        let collapsed = tree.visible_rows(&HashSet::new());
        assert_eq!(collapsed.len(), 2);

        let expanded: HashSet<PathBuf> =
            [PathBuf::from("/h"), PathBuf::from("/h/Fotos")].into_iter().collect();
        let rows: Vec<_> = tree
            .visible_rows(&expanded)
            .into_iter()
            .map(|(path, depth)| (path.to_string_lossy().into_owned(), depth))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("/h".to_string(), 0),
                ("/h/Descargas".to_string(), 1),
                ("/h/Fotos".to_string(), 1),
                ("/h/Fotos/2024".to_string(), 2),
                ("/h/Fotos/2025".to_string(), 2),
                ("/otro/lado".to_string(), 0),
            ]
        );
    }
}
