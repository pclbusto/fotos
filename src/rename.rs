//! Cambio de nombre de imágenes (F2) desde el navegador o el visor.

use std::path::{Path, PathBuf};

use cosmic::iced::Task;
use cosmic::Element;

use crate::app::AppModel;
use crate::browser::BrowserItem;
use crate::message::{Message, UiMessage};

pub fn input_id() -> cosmic::widget::Id {
    cosmic::widget::Id::new("rename-input")
}

/// Valida el nombre nuevo y devuelve la ruta destino, `Ok(None)` si no cambió.
fn target_path(original: &Path, new_name: &str) -> Result<Option<PathBuf>, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("El nombre no puede estar vacío".to_string());
    }
    if new_name == "." || new_name == ".." || new_name.contains('/') || new_name.contains('\0') {
        return Err("El nombre no puede contener «/» ni ser «.» o «..»".to_string());
    }
    let target = original.with_file_name(new_name);
    if target == original {
        return Ok(None);
    }
    // `exists` también protege contra pisar otro archivo; un cambio solo de
    // mayúsculas en sistemas de archivos insensibles apunta al mismo archivo.
    if target.exists() && !same_file(original, &target) {
        return Err(format!("Ya existe un archivo llamado «{new_name}»"));
    }
    Ok(Some(target))
}

fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

impl AppModel {
    pub(crate) fn rename_dialog(&self) -> Option<Element<'_, Message>> {
        let rename = self.ui.pending_rename.as_ref()?;
        let validation = target_path(&rename.path, &rename.name);
        let mut confirm = cosmic::widget::button::suggested("Cambiar nombre");
        if validation.is_ok() {
            confirm = confirm.on_press(Message::Ui(UiMessage::ConfirmRename));
        }

        let mut control = cosmic::iced::widget::Column::new().spacing(8).push(
            cosmic::widget::text_input("Nombre del archivo", &rename.name)
                .id(input_id())
                .on_input(|name| Message::Ui(UiMessage::RenameInputChanged(name)))
                .on_submit(|_| Message::Ui(UiMessage::ConfirmRename)),
        );
        if let Err(error) = validation {
            control = control.push(cosmic::widget::text::caption(error));
        }

        Some(
            cosmic::widget::dialog()
                .title("Cambiar nombre")
                .icon(cosmic::widget::icon::from_name("edit-symbolic").size(48))
                .control(control)
                .primary_action(confirm)
                .secondary_action(
                    cosmic::widget::button::standard("Cancelar")
                        .on_press(Message::Ui(UiMessage::CancelRename)),
                )
                .into(),
        )
    }

    pub(crate) fn confirm_rename(&mut self) -> Task<cosmic::Action<Message>> {
        let Some(rename) = self.ui.pending_rename.as_ref() else {
            return Task::none();
        };
        let original = rename.path.clone();
        let target = match target_path(&original, &rename.name) {
            Ok(Some(target)) => target,
            Ok(None) => {
                self.ui.pending_rename = None;
                return Task::none();
            }
            // El diálogo ya muestra el error; se deja abierto para corregirlo.
            Err(_) => return Task::none(),
        };

        if let Err(error) = std::fs::rename(&original, &target) {
            self.ui.error_message = Some(format!("No se pudo cambiar el nombre: {error}"));
            return Task::none();
        }
        self.ui.pending_rename = None;
        let new_name = target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Navegador: se actualiza el elemento en su lugar y se reordena
        // manteniendo el foco sobre él.
        let mut renamed_in_browser = false;
        for item in &mut self.browser.items {
            let BrowserItem::Image { name, path, .. } = item;
            if *path == original {
                *path = target.clone();
                *name = new_name.clone();
                renamed_in_browser = true;
            }
        }
        if renamed_in_browser {
            crate::browser::sort::sort_items(&mut self.browser.items, self.browser.sort);
            if let Some(index) = self.browser.items.iter().position(|item| item.path() == target)
            {
                self.browser.focused_index = index;
            }
        }

        // Visor: lista de imágenes, caché y archivo abierto.
        for image in &mut self.viewer.images {
            if *image == original {
                *image = target.clone();
            }
        }
        if let Some(cached) = self.viewer.cache.remove(&original) {
            self.viewer.cache.insert(target.clone(), cached);
        }
        if self.viewer.opened_file_path.as_ref() == Some(&original) {
            self.viewer.opened_file_path = Some(target.clone());
            if self.ui.view_mode == crate::state::ViewMode::Viewer {
                self.core.set_header_title(new_name.clone());
            }
        }

        // Base de datos de imágenes.
        let mut task = Task::none();
        if self.browser.database.remove_path(&original) {
            self.browser.database.upsert_single_file(&target);
            task = crate::browser::folders::save_task(self.browser.database.clone());
        }

        self.ui.notification_message = Some(format!("Renombrada a «{new_name}»"));
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_names() {
        let original = Path::new("/tmp/fotos-no-existe/a.png");
        assert!(target_path(original, "  ").is_err());
        assert!(target_path(original, "x/y.png").is_err());
        assert!(target_path(original, "..").is_err());
        assert_eq!(target_path(original, "a.png"), Ok(None));
        assert_eq!(
            target_path(original, " b.png "),
            Ok(Some(PathBuf::from("/tmp/fotos-no-existe/b.png")))
        );
    }
}
