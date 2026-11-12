//! Atajos de teclado configurables.
//!
//! Cada [`Action`] puede tener hasta dos [`Binding`]s. Las teclas del teclado
//! numérico se distinguen de las de la fila superior (`Num1` frente a `1`),
//! de modo que un atajo puede responder a ambas usando sus dos teclas.

use std::collections::HashMap;
use std::path::PathBuf;

use cosmic::iced::keyboard::key::{Code, Named, Physical};
use cosmic::iced::keyboard::{Key, Modifiers};

/// Cantidad máxima de teclas por atajo.
pub const MAX_BINDINGS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    File,
    Edit,
    View,
    Help,
}

impl Category {
    pub const ALL: [Self; 4] = [Self::File, Self::Edit, Self::View, Self::Help];

    pub fn label(self) -> &'static str {
        match self {
            Self::File => "Archivo",
            Self::Edit => "Edición",
            Self::View => "Vista",
            Self::Help => "Ayuda",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    OpenFile,
    SaveCopy,
    Quit,
    Undo,
    Trash,
    DeletePermanent,
    GridView,
    ListView,
    ToggleMetadata,
    PreviousImage,
    NextImage,
    ZoomIn,
    ZoomOut,
    ZoomActual,
    Fullscreen,
    Back,
    Folders,
    About,
    Shortcuts,
}

impl Action {
    pub const ALL: [Self; 19] = [
        Self::OpenFile,
        Self::SaveCopy,
        Self::Quit,
        Self::Undo,
        Self::Trash,
        Self::DeletePermanent,
        Self::GridView,
        Self::ListView,
        Self::ToggleMetadata,
        Self::PreviousImage,
        Self::NextImage,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomActual,
        Self::Fullscreen,
        Self::Back,
        Self::Folders,
        Self::About,
        Self::Shortcuts,
    ];

    /// Identificador estable usado en el archivo de configuración.
    pub fn id(self) -> &'static str {
        match self {
            Self::OpenFile => "open_file",
            Self::SaveCopy => "save_copy",
            Self::Quit => "quit",
            Self::Undo => "undo",
            Self::Trash => "trash",
            Self::DeletePermanent => "delete_permanent",
            Self::GridView => "grid_view",
            Self::ListView => "list_view",
            Self::ToggleMetadata => "toggle_metadata",
            Self::PreviousImage => "previous_image",
            Self::NextImage => "next_image",
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::ZoomActual => "zoom_actual",
            Self::Fullscreen => "fullscreen",
            Self::Back => "back",
            Self::Folders => "folders",
            Self::About => "about",
            Self::Shortcuts => "shortcuts",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::OpenFile => "Abrir imagen",
            Self::SaveCopy => "Guardar una copia",
            Self::Quit => "Salir",
            Self::Undo => "Deshacer",
            Self::Trash => "Enviar a la papelera",
            Self::DeletePermanent => "Eliminar permanentemente",
            Self::GridView => "Vista de grilla",
            Self::ListView => "Vista de lista",
            Self::ToggleMetadata => "Información de la foto",
            Self::PreviousImage => "Imagen anterior",
            Self::NextImage => "Imagen siguiente",
            Self::ZoomIn => "Acercar",
            Self::ZoomOut => "Alejar",
            Self::ZoomActual => "Tamaño real",
            Self::Fullscreen => "Pantalla completa",
            Self::Back => "Volver",
            Self::Folders => "Carpetas",
            Self::About => "Acerca de Fotos",
            Self::Shortcuts => "Atajos de teclado",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::OpenFile => "Abre una imagen desde el selector de archivos",
            Self::SaveCopy => "Guarda la imagen editada en un archivo nuevo",
            Self::Quit => "Cierra Fotos y consulta antes si hay cambios",
            Self::Undo => "Deshace la última transformación",
            Self::Trash => "Elimina la imagen o carpeta seleccionada",
            Self::DeletePermanent => "Elimina la selección sin pasar por la papelera",
            Self::GridView => "Muestra el navegador como una grilla de miniaturas",
            Self::ListView => "Muestra el navegador como una lista detallada",
            Self::ToggleMetadata => "Muestra u oculta los metadatos de la imagen",
            Self::PreviousImage => "Abre la imagen anterior de la carpeta",
            Self::NextImage => "Abre la imagen siguiente de la carpeta",
            Self::ZoomIn => "Acerca la imagen",
            Self::ZoomOut => "Aleja la imagen",
            Self::ZoomActual => "Muestra la imagen al 100 %",
            Self::Fullscreen => "Activa o desactiva la pantalla completa",
            Self::Back => "Vuelve del visor al navegador",
            Self::Folders => "Muestra u oculta las carpetas escaneadas",
            Self::About => "Muestra la versión, autoría, licencia y enlaces",
            Self::Shortcuts => "Muestra u oculta este panel",
        }
    }

    pub fn category(self) -> Category {
        match self {
            Self::OpenFile | Self::SaveCopy | Self::Quit => Category::File,
            Self::Undo | Self::Trash | Self::DeletePermanent => Category::Edit,
            Self::About | Self::Shortcuts => Category::Help,
            _ => Category::View,
        }
    }

    fn default_bindings(self) -> Vec<Binding> {
        let ctrl = |key: &str| Binding::new(true, false, false, key);
        let plain = |key: &str| Binding::new(false, false, false, key);
        match self {
            Self::OpenFile => vec![ctrl("o")],
            Self::SaveCopy => vec![ctrl("s")],
            Self::Quit => vec![ctrl("q"), ctrl("w")],
            Self::Undo => vec![ctrl("z")],
            Self::Trash => vec![plain("Delete")],
            Self::DeletePermanent => vec![ctrl("Delete")],
            Self::GridView => vec![ctrl("1")],
            Self::ListView => vec![ctrl("2")],
            Self::ToggleMetadata => vec![ctrl("Space")],
            Self::PreviousImage => vec![plain("ArrowLeft")],
            Self::NextImage => vec![plain("ArrowRight")],
            Self::ZoomIn => vec![plain("Plus"), plain("NumPlus")],
            Self::ZoomOut => vec![plain("Minus"), plain("NumMinus")],
            Self::ZoomActual => vec![plain("1"), plain("Num1")],
            Self::Fullscreen => vec![plain("F11")],
            Self::Back => vec![plain("Backspace")],
            Self::Folders => vec![ctrl("p")],
            Self::About => vec![plain("F1")],
            Self::Shortcuts => vec![ctrl("F1")],
        }
    }
}

/// Una combinación de modificadores más una tecla.
///
/// `key` es un nombre normalizado sin espacios ni `+`
/// (por ejemplo `o`, `1`, `Num1`, `Plus`, `F11`, `ArrowLeft`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Binding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

impl Binding {
    pub fn new(ctrl: bool, alt: bool, shift: bool, key: &str) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            key: key.to_string(),
        }
    }

    /// Construye la combinación correspondiente a una pulsación. Devuelve
    /// `None` para teclas que no se pueden asignar (modificadores sueltos,
    /// Escape, Tab, etc.).
    pub fn from_event(key: &Key, physical: &Physical, modifiers: Modifiers) -> Option<Self> {
        let mut shift = modifiers.shift();
        let name = if let Some(name) = numpad_name(physical) {
            name.to_string()
        } else {
            match key {
                Key::Named(named) => named_name(*named)?,
                Key::Character(c) => {
                    let lower = c.to_lowercase();
                    match lower.as_str() {
                        " " => "Space".to_string(),
                        "+" => {
                            shift = false;
                            "Plus".to_string()
                        }
                        "-" => {
                            shift = false;
                            "Minus".to_string()
                        }
                        _ => {
                            // El carácter ya refleja Shift salvo en letras.
                            if !lower.chars().all(char::is_alphabetic) {
                                shift = false;
                            }
                            if lower.chars().any(|c| c.is_whitespace() || c == '+') {
                                return None;
                            }
                            lower
                        }
                    }
                }
                Key::Unidentified => return None,
            }
        };

        Some(Self {
            ctrl: modifiers.control(),
            alt: modifiers.alt(),
            shift,
            key: name,
        })
    }

    /// Texto de cada tecla para mostrar como teclas individuales.
    pub fn labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        if self.ctrl {
            labels.push("Ctrl".to_string());
        }
        if self.alt {
            labels.push("Alt".to_string());
        }
        if self.shift {
            labels.push("Mayús".to_string());
        }
        labels.push(key_label(&self.key));
        labels
    }

    fn serialize(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }

    fn parse(text: &str) -> Option<Self> {
        let mut binding = Self::new(false, false, false, "");
        let mut parts = text.split('+').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                if part.is_empty() {
                    return None;
                }
                binding.key = part.to_string();
            } else {
                match part {
                    "Ctrl" => binding.ctrl = true,
                    "Alt" => binding.alt = true,
                    "Shift" => binding.shift = true,
                    _ => return None,
                }
            }
        }
        Some(binding)
    }
}

fn numpad_name(physical: &Physical) -> Option<&'static str> {
    let Physical::Code(code) = physical else {
        return None;
    };
    Some(match code {
        Code::Numpad0 => "Num0",
        Code::Numpad1 => "Num1",
        Code::Numpad2 => "Num2",
        Code::Numpad3 => "Num3",
        Code::Numpad4 => "Num4",
        Code::Numpad5 => "Num5",
        Code::Numpad6 => "Num6",
        Code::Numpad7 => "Num7",
        Code::Numpad8 => "Num8",
        Code::Numpad9 => "Num9",
        Code::NumpadAdd => "NumPlus",
        Code::NumpadSubtract => "NumMinus",
        Code::NumpadMultiply => "NumStar",
        Code::NumpadDivide => "NumSlash",
        Code::NumpadDecimal => "NumDot",
        Code::NumpadEnter => "NumEnter",
        _ => return None,
    })
}

fn named_name(named: Named) -> Option<String> {
    match named {
        Named::Shift
        | Named::Control
        | Named::Alt
        | Named::AltGraph
        | Named::Super
        | Named::Meta
        | Named::Hyper
        | Named::Fn
        | Named::FnLock
        | Named::CapsLock
        | Named::NumLock
        | Named::ScrollLock
        | Named::Symbol
        | Named::SymbolLock
        | Named::Escape
        | Named::Tab => None,
        other => Some(format!("{other:?}")),
    }
}

fn key_label(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("Num") {
        if !rest.is_empty() {
            let symbol = match rest {
                "Plus" => "+",
                "Minus" => "−",
                "Star" => "*",
                "Slash" => "/",
                "Dot" => ".",
                "Enter" => "Intro",
                other => other,
            };
            return format!("Num {symbol}");
        }
    }
    match key {
        "Plus" => "+".to_string(),
        "Minus" => "−".to_string(),
        "Space" => "Espacio".to_string(),
        "Delete" => "Supr".to_string(),
        "Backspace" => "Retroceso".to_string(),
        "Enter" => "Intro".to_string(),
        "Insert" => "Insert".to_string(),
        "Home" => "Inicio".to_string(),
        "End" => "Fin".to_string(),
        "PageUp" => "RePág".to_string(),
        "PageDown" => "AvPág".to_string(),
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        other if other.chars().count() == 1 => other.to_uppercase(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: HashMap<Action, Vec<Binding>>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            bindings: Action::ALL
                .into_iter()
                .map(|action| (action, action.default_bindings()))
                .collect(),
        }
    }
}

impl Keymap {
    pub fn bindings(&self, action: Action) -> &[Binding] {
        self.bindings.get(&action).map_or(&[], Vec::as_slice)
    }

    /// Acciones asociadas a una combinación, en orden de declaración.
    pub fn actions_for(&self, binding: &Binding) -> impl Iterator<Item = Action> + '_ {
        let binding = binding.clone();
        Action::ALL
            .into_iter()
            .filter(move |action| self.bindings(*action).contains(&binding))
    }

    /// Asigna las teclas a una acción. Si alguna ya estaba en uso por otra
    /// acción, se la quita de esa otra y devuelve las acciones afectadas.
    pub fn assign(&mut self, action: Action, mut bindings: Vec<Binding>) -> Vec<Action> {
        bindings.dedup();
        bindings.truncate(MAX_BINDINGS);
        let mut displaced = Vec::new();
        for (other, other_bindings) in self.bindings.iter_mut() {
            if *other == action {
                continue;
            }
            let before = other_bindings.len();
            other_bindings.retain(|binding| !bindings.contains(binding));
            if other_bindings.len() != before {
                displaced.push(*other);
            }
        }
        self.bindings.insert(action, bindings);
        displaced
    }

    pub fn is_default(&self) -> bool {
        Action::ALL
            .into_iter()
            .all(|action| self.bindings(action) == action.default_bindings().as_slice())
    }

    pub fn load() -> Self {
        keymap_path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|contents| Self::parse(&contents))
            .unwrap_or_default()
    }

    fn parse(contents: &str) -> Self {
        let mut keymap = Self::default();
        for line in contents.lines() {
            let Some((id, value)) = line.split_once('=') else {
                continue;
            };
            let Some(action) = Action::from_id(id.trim()) else {
                continue;
            };
            let bindings = value
                .split_whitespace()
                .filter_map(Binding::parse)
                .take(MAX_BINDINGS)
                .collect();
            keymap.bindings.insert(action, bindings);
        }
        keymap
    }

    fn serialize(&self) -> String {
        Action::ALL
            .into_iter()
            .map(|action| {
                let value = self
                    .bindings(action)
                    .iter()
                    .map(Binding::serialize)
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{}={value}\n", action.id())
            })
            .collect()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = keymap_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.serialize())
    }
}

fn keymap_path() -> Option<PathBuf> {
    dirs::config_dir().map(|directory| directory.join("fotos").join("shortcuts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpad_and_top_row_are_distinct() {
        let top = Binding::from_event(
            &Key::Character("1".into()),
            &Physical::Code(Code::Digit1),
            Modifiers::default(),
        )
        .unwrap();
        let numpad = Binding::from_event(
            &Key::Character("1".into()),
            &Physical::Code(Code::Numpad1),
            Modifiers::default(),
        )
        .unwrap();
        assert_eq!(top.key, "1");
        assert_eq!(numpad.key, "Num1");

        let keymap = Keymap::default();
        assert_eq!(keymap.actions_for(&top).next(), Some(Action::ZoomActual));
        assert_eq!(keymap.actions_for(&numpad).next(), Some(Action::ZoomActual));
    }

    #[test]
    fn numpad_works_without_numlock() {
        let binding = Binding::from_event(
            &Key::Named(Named::End),
            &Physical::Code(Code::Numpad1),
            Modifiers::default(),
        )
        .unwrap();
        assert_eq!(binding.key, "Num1");
    }

    #[test]
    fn round_trips_through_file_format() {
        let mut keymap = Keymap::default();
        keymap.assign(
            Action::ZoomIn,
            vec![
                Binding::new(true, false, true, "Plus"),
                Binding::new(false, false, false, "NumPlus"),
            ],
        );
        let parsed = Keymap::parse(&keymap.serialize());
        assert_eq!(parsed.bindings(Action::ZoomIn), keymap.bindings(Action::ZoomIn));
        assert_eq!(parsed.bindings(Action::Quit), keymap.bindings(Action::Quit));
    }

    #[test]
    fn assigning_steals_from_other_actions() {
        let mut keymap = Keymap::default();
        let f1 = Binding::new(false, false, false, "F1");
        let displaced = keymap.assign(Action::ZoomActual, vec![f1.clone()]);
        assert_eq!(displaced, vec![Action::About]);
        assert!(keymap.bindings(Action::About).is_empty());
        assert_eq!(keymap.bindings(Action::ZoomActual), &[f1]);
    }
}
