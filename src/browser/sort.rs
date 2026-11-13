//! Criterios de ordenamiento de las imágenes del navegador y del visor.

use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::SystemTime;

use super::BrowserItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Orden natural por nombre: `img2` va antes que `img10`.
    #[default]
    Name,
    /// Fecha de modificación del archivo.
    Modified,
    /// Tamaño del archivo.
    Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SortOrder {
    pub key: SortKey,
    pub descending: bool,
}

impl SortKey {
    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Modified => "modified",
            Self::Size => "size",
        }
    }

    pub fn from_key(value: &str) -> Self {
        match value {
            "modified" => Self::Modified,
            "size" => Self::Size,
            _ => Self::Name,
        }
    }
}

/// Compara cadenas tratando las secuencias de dígitos como números y el resto
/// sin distinguir mayúsculas.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();
    loop {
        match (a_chars.peek().copied(), b_chars.peek().copied()) {
            (None, None) => break,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take_digits = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = chars.peek().copied().filter(char::is_ascii_digit) {
                        digits.push(c);
                        chars.next();
                    }
                    digits
                };
                let x_digits = take_digits(&mut a_chars);
                let y_digits = take_digits(&mut b_chars);
                let x_trimmed = x_digits.trim_start_matches('0');
                let y_trimmed = y_digits.trim_start_matches('0');
                let ordering = x_trimmed
                    .len()
                    .cmp(&y_trimmed.len())
                    .then_with(|| x_trimmed.cmp(y_trimmed))
                    // Con el mismo valor, menos ceros a la izquierda primero.
                    .then_with(|| x_digits.len().cmp(&y_digits.len()));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(x), Some(y)) => {
                let ordering = x.to_lowercase().cmp(y.to_lowercase());
                if ordering != Ordering::Equal {
                    return ordering;
                }
                a_chars.next();
                b_chars.next();
            }
        }
    }
    // Desempate determinista entre nombres que solo difieren en mayúsculas.
    a.cmp(b)
}

fn compare_fields(
    order: SortOrder,
    (a_name, a_modified, a_size): (&str, Option<SystemTime>, Option<u64>),
    (b_name, b_modified, b_size): (&str, Option<SystemTime>, Option<u64>),
) -> Ordering {
    let by_name = natural_cmp(a_name, b_name);
    let primary = match order.key {
        SortKey::Name => by_name,
        SortKey::Modified => a_modified.cmp(&b_modified),
        SortKey::Size => a_size.cmp(&b_size),
    };
    let primary = if order.descending {
        primary.reverse()
    } else {
        primary
    };
    primary.then(by_name)
}

pub fn sort_items(items: &mut [BrowserItem], order: SortOrder) {
    items.sort_by(|a, b| {
        compare_fields(
            order,
            (a.name(), a.modified(), a.file_size()),
            (b.name(), b.modified(), b.file_size()),
        )
    });
}

/// Ordena rutas sueltas (p. ej. las hermanas de un archivo abierto con el
/// diálogo) leyendo sus metadatos del disco.
pub fn sort_paths(paths: &mut Vec<PathBuf>, order: SortOrder) {
    let mut entries: Vec<(PathBuf, String, Option<SystemTime>, Option<u64>)> = paths
        .drain(..)
        .map(|path| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let metadata = std::fs::metadata(&path).ok();
            let modified = metadata.as_ref().and_then(|m| m.modified().ok());
            let size = metadata.map(|m| m.len());
            (path, name, modified, size)
        })
        .collect();
    entries.sort_by(|a, b| compare_fields(order, (&a.1, a.2, a.3), (&b.1, b.2, b.3)));
    paths.extend(entries.into_iter().map(|(path, ..)| path));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_compares_numbers_by_value() {
        let mut names = vec!["img10.png", "IMG2.png", "img1.png", "img02.png", "foto.jpg"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec!["foto.jpg", "img1.png", "IMG2.png", "img02.png", "img10.png"]
        );
    }

    #[test]
    fn sort_key_roundtrips() {
        for key in [SortKey::Name, SortKey::Modified, SortKey::Size] {
            assert_eq!(SortKey::from_key(key.key()), key);
        }
    }
}
