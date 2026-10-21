use std::path::PathBuf;

use cosmic::iced::Task;

use crate::codecs;
use crate::editor;
use crate::message::{Message, ViewerMessage};
use crate::viewer::{CachedImage, ImageMetadata};

pub fn preload_task(path: PathBuf) -> Task<cosmic::Action<Message>> {
    let preload_path = path.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || match codecs::open(&preload_path) {
                Ok(image) => {
                    let cached = CachedImage {
                        handle: editor::to_handle(&image),
                        width: image.width() as f32,
                        height: image.height() as f32,
                    };
                    Ok((preload_path, cached))
                }
                Err(error) => Err((preload_path, error.to_string())),
            })
            .await
            .unwrap_or_else(|_| Err((path, "Error al iniciar el hilo de precarga".to_string())))
        },
        |result| cosmic::Action::App(Message::Viewer(ViewerMessage::PreloadCompleted(result))),
    )
}

pub fn load_editable_task(path: PathBuf) -> Task<cosmic::Action<Message>> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || codecs::open(&path).map(|image| (path, image)))
                .await
                .unwrap_or_else(|_| {
                    Err(image::ImageError::IoError(std::io::Error::other(
                        "Thread spawn failed",
                    )))
                })
        },
        |result| {
            cosmic::Action::App(Message::Viewer(ViewerMessage::ActiveImageLoaded(
                result.map_err(|e| e.to_string()),
            )))
        },
    )
}

pub fn metadata_task(path: PathBuf) -> Task<cosmic::Action<Message>> {
    let metadata_path = path.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || read_metadata(&metadata_path))
                .await
                .unwrap_or_default()
        },
        move |metadata| {
            cosmic::Action::App(Message::Viewer(ViewerMessage::MetadataLoaded(
                path.clone(),
                metadata,
            )))
        },
    )
}

fn read_metadata(path: &std::path::Path) -> ImageMetadata {
    use exif::{In, Reader, Tag};
    use std::io::BufReader;

    let file_size = std::fs::metadata(path).ok().map(|metadata| metadata.len());
    let Ok(file) = std::fs::File::open(path) else {
        return ImageMetadata {
            file_size,
            fields: Vec::new(),
        };
    };
    let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(file)) else {
        return ImageMetadata {
            file_size,
            fields: Vec::new(),
        };
    };

    let mut fields = Vec::new();
    let mut add = |label: &str, tag: Tag| {
        if let Some(field) = exif.get_field(tag, In::PRIMARY) {
            let value = field.display_value().with_unit(&exif).to_string();
            if !value.trim().is_empty() {
                fields.push((label.to_string(), value));
            }
        }
    };

    add("Fabricante", Tag::Make);
    add("Cámara", Tag::Model);
    add("Fecha de captura", Tag::DateTimeOriginal);
    add("Orientación", Tag::Orientation);
    add("Exposición", Tag::ExposureTime);
    add("Apertura", Tag::FNumber);
    add("ISO", Tag::PhotographicSensitivity);
    add("Distancia focal", Tag::FocalLength);

    ImageMetadata { file_size, fields }
}
