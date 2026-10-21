use cosmic::Element;

use crate::app::AppModel;
use crate::message::{Message, UiMessage, ViewerMessage};

impl AppModel {
    pub(crate) fn dialog_view(&self) -> Option<Element<'_, Message>> {
        if let Some(request) = &self.ui.pending_delete {
            let name = request
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| request.path.to_string_lossy().into_owned());
            let kind = if request.is_dir {
                "la carpeta"
            } else {
                "la imagen"
            };
            let body = if request.permanent {
                format!(
                    "Se eliminará {kind} «{name}» de forma permanente. Esta acción no se puede deshacer."
                )
            } else {
                format!("Se moverá {kind} «{name}» a la papelera.")
            };

            return Some(
                cosmic::widget::dialog()
                    .title("¿Eliminar elemento?")
                    .body(body)
                    .icon(cosmic::widget::icon::from_name("dialog-warning-symbolic").size(48))
                    .primary_action(
                        cosmic::widget::button::destructive(if request.permanent {
                            "Eliminar permanentemente"
                        } else {
                            "Mover a la papelera"
                        })
                        .on_press(Message::Ui(UiMessage::ConfirmDelete)),
                    )
                    .secondary_action(
                        cosmic::widget::button::standard("Cancelar")
                            .on_press(Message::Ui(UiMessage::CancelDelete)),
                    )
                    .into(),
            );
        }

        self.ui.pending_action.as_ref()?;
        Some(
            cosmic::widget::dialog()
                .title("La imagen tiene cambios sin guardar")
                .body("Elegí qué hacer con los cambios antes de continuar.")
                .icon(cosmic::widget::icon::from_name("dialog-warning-symbolic").size(48))
                .primary_action(
                    cosmic::widget::button::suggested("Guardar copia")
                        .on_press(Message::Viewer(ViewerMessage::SaveCopy)),
                )
                .secondary_action(
                    cosmic::widget::button::standard("Sobrescribir")
                        .on_press(Message::Viewer(ViewerMessage::SaveOverwrite)),
                )
                .tertiary_action(
                    cosmic::widget::button::destructive("Descartar")
                        .on_press(Message::Ui(UiMessage::DiscardChanges)),
                )
                .into(),
        )
    }
}
