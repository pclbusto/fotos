//! Miniaturas según la especificación de freedesktop.org
//! (<https://specifications.freedesktop.org/thumbnail-spec/latest/>).
//!
//! Las miniaturas se comparten con COSMIC Files, Nautilus, Dolphin, etc. en
//! `$XDG_CACHE_HOME/thumbnails/{normal,large,x-large,xx-large}/<md5(uri)>.png`.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::BufWriter;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use image::DynamicImage;
use md5::{Digest, Md5};

use crate::codecs;

/// Tamaño que genera la app: `large` (256 px), nítido también en pantallas HiDPI.
const GENERATED_FLAVOR: Flavor = Flavor::Large;

/// Tamaño máximo que se mantiene en memoria para la grilla.
const DISPLAY_SIZE: u32 = 256;

#[derive(Debug, Clone, Copy)]
enum Flavor {
    Large,
    XLarge,
    XXLarge,
}

impl Flavor {
    fn dir_name(self) -> &'static str {
        match self {
            Flavor::Large => "large",
            Flavor::XLarge => "x-large",
            Flavor::XXLarge => "xx-large",
        }
    }

    fn size(self) -> u32 {
        match self {
            Flavor::Large => 256,
            Flavor::XLarge => 512,
            Flavor::XXLarge => 1024,
        }
    }
}

/// Orden de búsqueda: primero el tamaño justo, después los más grandes (que se reducen).
const LOOKUP_ORDER: [Flavor; 3] = [Flavor::Large, Flavor::XLarge, Flavor::XXLarge];

/// Devuelve la miniatura de `path`: la reutiliza del caché compartido si está
/// vigente o la genera (y la guarda en `large/`) si no.
pub fn load(path: &Path) -> Option<DynamicImage> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let metadata = fs::metadata(&path).ok()?;
    let mtime = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let uri = file_uri(&path);
    let file_name = format!("{}.png", md5_hex(&uri));
    let base = thumbnails_dir();

    if let Some(base) = &base {
        for flavor in LOOKUP_ORDER {
            let candidate = base.join(flavor.dir_name()).join(&file_name);
            if let Some(image) = read_valid(&candidate, &uri, mtime) {
                return Some(fit(image, DISPLAY_SIZE));
            }
        }
    }

    let original = codecs::open(&path).ok()?;
    let (width, height) = (original.width(), original.height());
    let thumbnail = fit(original, GENERATED_FLAVOR.size());

    if let Some(base) = &base {
        let info = ThumbInfo {
            uri: &uri,
            mtime,
            file_size: metadata.len(),
            width,
            height,
        };
        let _ = write(
            &base.join(GENERATED_FLAVOR.dir_name()),
            &file_name,
            &thumbnail,
            &info,
        );
    }

    Some(fit(thumbnail, DISPLAY_SIZE))
}

fn thumbnails_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|cache| cache.join("thumbnails"))
}

/// Reduce la imagen para que entre en `max`×`max` sin agrandarla nunca.
fn fit(image: DynamicImage, max: u32) -> DynamicImage {
    if image.width() > max || image.height() > max {
        image.thumbnail(max, max)
    } else {
        image
    }
}

/// URI `file://` con el mismo escapado que `g_filename_to_uri` de GLib, que
/// es lo que usan los demás programas para calcular el nombre del archivo.
fn file_uri(path: &Path) -> String {
    const SAFE: &[u8] = b"-_.!~*'()/$&+,:=@";
    let mut uri = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || SAFE.contains(&byte) {
            uri.push(byte as char);
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri
}

fn md5_hex(text: &str) -> String {
    Md5::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Lee los chunks `tEXt` de un PNG (estén antes o después de los datos de imagen).
fn png_text_chunks(data: &[u8]) -> Vec<(String, String)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    let mut chunks = Vec::new();
    if !data.starts_with(SIGNATURE) {
        return chunks;
    }
    let mut offset = SIGNATURE.len();
    while offset + 8 <= data.len() {
        let length = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &data[offset + 4..offset + 8];
        let body_start = offset + 8;
        let Some(body_end) = body_start.checked_add(length).filter(|end| *end <= data.len())
        else {
            break;
        };
        let body = &data[body_start..body_end];
        if kind == b"tEXt" {
            if let Some(separator) = body.iter().position(|byte| *byte == 0) {
                // tEXt es Latin-1
                let latin1 = |bytes: &[u8]| bytes.iter().map(|&b| b as char).collect::<String>();
                chunks.push((latin1(&body[..separator]), latin1(&body[separator + 1..])));
            }
        } else if kind == b"IEND" {
            break;
        }
        offset = body_end + 4; // + CRC
    }
    chunks
}

fn read_valid(candidate: &Path, uri: &str, mtime: u64) -> Option<DynamicImage> {
    let data = fs::read(candidate).ok()?;
    let chunks = png_text_chunks(&data);
    let value = |key: &str| {
        chunks
            .iter()
            .find(|(keyword, _)| keyword == key)
            .map(|(_, text)| text.as_str())
    };

    if value("Thumb::URI")? != uri {
        return None;
    }
    // Algunos programas escriben fracciones de segundo; se compara la parte entera.
    let stored_mtime: u64 = value("Thumb::MTime")?.split('.').next()?.trim().parse().ok()?;
    if stored_mtime != mtime {
        return None;
    }

    image::load_from_memory_with_format(&data, image::ImageFormat::Png).ok()
}

struct ThumbInfo<'a> {
    uri: &'a str,
    mtime: u64,
    file_size: u64,
    width: u32,
    height: u32,
}

/// Guarda la miniatura de forma atómica (archivo temporal + rename) con los
/// permisos que pide la especificación: 0700 para carpetas, 0600 para archivos.
fn write(
    dir: &Path,
    file_name: &str,
    thumbnail: &DynamicImage,
    info: &ThumbInfo<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)?;

    let final_path = dir.join(file_name);
    let temp_path = dir.join(format!("{file_name}.{}.tmp", std::process::id()));

    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp_path)?;
        let rgba = thumbnail.to_rgba8();
        let mut encoder = png::Encoder::new(BufWriter::new(file), rgba.width(), rgba.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.add_text_chunk("Thumb::URI".into(), info.uri.into())?;
        encoder.add_text_chunk("Thumb::MTime".into(), info.mtime.to_string())?;
        encoder.add_text_chunk("Thumb::Size".into(), info.file_size.to_string())?;
        encoder.add_text_chunk("Thumb::Image::Width".into(), info.width.to_string())?;
        encoder.add_text_chunk("Thumb::Image::Height".into(), info.height.to_string())?;
        encoder.add_text_chunk("Software".into(), "visor_fotos".into())?;
        let mut writer = encoder.write_header()?;
        writer.write_image_data(rgba.as_raw())?;
        writer.finish()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            fs::rename(&temp_path, &final_path)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_uri_like_glib() {
        assert_eq!(
            file_uri(Path::new("/home/user/Fotos/año nuevo (1)&co.jpg")),
            "file:///home/user/Fotos/a%C3%B1o%20nuevo%20(1)&co.jpg"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/a#b?c%d.png")),
            "file:///tmp/a%23b%3Fc%25d.png"
        );
    }

    #[test]
    fn md5_matches_spec_example() {
        // Ejemplo de la especificación freedesktop.
        assert_eq!(
            md5_hex("file:///home/jens/photos/me.png"),
            "c6ee772d9e49320e97ec29a7eb5b1697"
        );
    }

    #[test]
    fn written_thumbnail_round_trips() {
        let dir = std::env::temp_dir().join(format!("fotos-thumb-test-{}", std::process::id()));
        let image = DynamicImage::new_rgba8(10, 8);
        let info = ThumbInfo {
            uri: "file:///x/y.png",
            mtime: 1234,
            file_size: 99,
            width: 10,
            height: 8,
        };
        write(&dir, "t.png", &image, &info).unwrap();
        let path = dir.join("t.png");
        assert!(read_valid(&path, "file:///x/y.png", 1234).is_some());
        assert!(read_valid(&path, "file:///x/y.png", 1235).is_none());
        assert!(read_valid(&path, "file:///x/z.png", 1234).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
