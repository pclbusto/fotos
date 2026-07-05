use std::path::PathBuf;

use cosmic::iced::Task;

use crate::codecs;
use crate::editor;
use crate::message::{Message, ViewerMessage};
use crate::viewer::CachedImage;

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
                    Err(image::ImageError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
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
