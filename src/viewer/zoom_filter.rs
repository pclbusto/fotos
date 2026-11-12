use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cosmic::iced::advanced::image::Id;
use cosmic::iced::widget::image::{FilterMethod, Handle};
use cosmic::iced::Task;
use image::imageops::FilterType;
use image::DynamicImage;

use crate::app::AppModel;
use crate::editor;
use crate::message::{Message, ViewerMessage};
use crate::state::ViewMode;

/// Largest image (in pixels) we are willing to resample on the CPU for display.
/// Beyond this the GPU bilinear filter is used as a fallback.
const MAX_RESAMPLED_PIXELS: u64 = 32_000_000;
/// Wait for zoom changes to settle before resampling, so fast zooming stays fluid.
const RESAMPLE_DEBOUNCE: Duration = Duration::from_millis(150);

/// Interpolation used to draw the image when it is zoomed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomFilter {
    /// No interpolation: every image pixel is drawn as a sharp block.
    None,
    #[default]
    Bilinear,
    Bicubic,
    Lanczos,
}

pub const ZOOM_FILTER_LABELS: [&str; 4] = [
    "Ninguno (píxeles nítidos)",
    "Bilineal (rápido)",
    "Bicúbico",
    "Lanczos (máxima calidad)",
];

impl ZoomFilter {
    const ALL: [Self; 4] = [Self::None, Self::Bilinear, Self::Bicubic, Self::Lanczos];

    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|filter| *filter == self)
            .unwrap_or(1)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Bilinear => "bilinear",
            Self::Bicubic => "bicubic",
            Self::Lanczos => "lanczos",
        }
    }

    pub fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|filter| filter.key() == key)
            .unwrap_or_default()
    }

    /// Sampler used by the GPU when drawing the image handle.
    pub fn gpu_filter(self) -> FilterMethod {
        match self {
            Self::None => FilterMethod::Nearest,
            _ => FilterMethod::Linear,
        }
    }

    /// Filters the GPU cannot do are resampled on the CPU.
    fn cpu_filter(self) -> Option<FilterType> {
        match self {
            Self::Bicubic => Some(FilterType::CatmullRom),
            Self::Lanczos => Some(FilterType::Lanczos3),
            Self::None | Self::Bilinear => None,
        }
    }
}

/// Identifies one CPU-resampled rendering of the displayed image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResampleKey {
    source: Id,
    width: u32,
    height: u32,
    filter: ZoomFilter,
}

#[derive(Default)]
pub struct ZoomRenderState {
    /// Shared copy of the source pixels, reused across zoom levels of one image.
    source: Option<(Id, Arc<DynamicImage>)>,
    rendered: Option<(ResampleKey, Handle)>,
    pending: Option<ResampleKey>,
    generation: Arc<AtomicU64>,
}

impl AppModel {
    fn desired_resample_key(&self) -> Option<ResampleKey> {
        let filter = self.settings.zoom_filter;
        filter.cpu_filter()?;
        if self.ui.view_mode != ViewMode::Viewer || self.editor.scale_preview_active {
            return None;
        }
        let handle = self.viewer.image_handle.as_ref()?;
        let image = self.viewer.original_image.as_ref()?;
        if image.width() as f32 != self.viewer.image_width
            || image.height() as f32 != self.viewer.image_height
        {
            return None;
        }

        let zoom = self.viewer.zoom_factor;
        let width = (self.viewer.image_width * zoom).round().max(1.0) as u32;
        let height = (self.viewer.image_height * zoom).round().max(1.0) as u32;
        if (width == image.width() && height == image.height())
            || u64::from(width) * u64::from(height) > MAX_RESAMPLED_PIXELS
        {
            return None;
        }

        Some(ResampleKey {
            source: handle.id(),
            width,
            height,
            filter,
        })
    }

    /// Handle to draw in the viewer: the CPU-resampled one when it is up to date,
    /// otherwise the original (scaled by the GPU in the meantime).
    pub(crate) fn display_handle(&self) -> Option<Handle> {
        let original = self.viewer.image_handle.as_ref()?;
        match (
            &self.viewer.zoom_render.rendered,
            self.desired_resample_key(),
        ) {
            (Some((key, handle)), Some(desired)) if *key == desired => Some(handle.clone()),
            _ => Some(original.clone()),
        }
    }

    /// Starts (or cancels) background resampling so it matches the current zoom.
    pub(crate) fn sync_zoom_render(&mut self) -> Task<cosmic::Action<Message>> {
        let desired = self.desired_resample_key();
        let keep_source = self.settings.zoom_filter.cpu_filter().is_some()
            && self.ui.view_mode == ViewMode::Viewer
            && self.viewer.image_handle.is_some();
        let render = &mut self.viewer.zoom_render;
        let Some(key) = desired else {
            render.pending = None;
            render.generation.fetch_add(1, Ordering::Relaxed);
            if !keep_source {
                render.source = None;
                render.rendered = None;
            }
            return Task::none();
        };

        if render
            .rendered
            .as_ref()
            .is_some_and(|(done, _)| *done == key)
            || render.pending == Some(key)
        {
            return Task::none();
        }

        let source = match &render.source {
            Some((id, image)) if *id == key.source => image.clone(),
            _ => {
                let Some(image) = self.viewer.original_image.as_ref() else {
                    return Task::none();
                };
                let image = Arc::new(image.clone());
                render.source = Some((key.source, image.clone()));
                image
            }
        };

        let Some(filter) = key.filter.cpu_filter() else {
            return Task::none();
        };
        let generation = render.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let counter = render.generation.clone();
        render.pending = Some(key);

        Task::perform(
            async move {
                tokio::time::sleep(RESAMPLE_DEBOUNCE).await;
                if counter.load(Ordering::Relaxed) != generation {
                    return None;
                }
                tokio::task::spawn_blocking(move || {
                    if counter.load(Ordering::Relaxed) != generation {
                        return None;
                    }
                    let resized = source.resize_exact(key.width, key.height, filter);
                    Some(editor::to_handle(&resized))
                })
                .await
                .ok()
                .flatten()
            },
            move |handle| {
                cosmic::Action::App(Message::Viewer(ViewerMessage::ZoomRenderReady(key, handle)))
            },
        )
    }

    pub(crate) fn zoom_render_ready(&mut self, key: ResampleKey, handle: Option<Handle>) {
        let render = &mut self.viewer.zoom_render;
        if render.pending != Some(key) {
            return;
        }
        render.pending = None;
        if let Some(handle) = handle {
            render.rendered = Some((key, handle));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_filter_round_trips_through_preferences_and_index() {
        for (index, filter) in ZoomFilter::ALL.into_iter().enumerate() {
            assert_eq!(ZoomFilter::from_key(filter.key()), filter);
            assert_eq!(ZoomFilter::from_index(index), filter);
            assert_eq!(filter.index(), index);
        }
        assert_eq!(ZoomFilter::from_key("desconocido"), ZoomFilter::Bilinear);
    }

    #[test]
    fn none_disables_gpu_interpolation() {
        assert_eq!(ZoomFilter::None.gpu_filter(), FilterMethod::Nearest);
        assert_eq!(ZoomFilter::Bilinear.gpu_filter(), FilterMethod::Linear);
    }
}
