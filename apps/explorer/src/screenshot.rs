//! One-shot PNG evidence capture (stage 5b). Rendering lives in svf-render;
//! this module only encodes and writes the captured frame.

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use svf_render::{CapturedImage, SceneRenderer};

/// Capture size. Fixed so a command line gives the same image on any machine,
/// independent of the window or display scaling.
pub const CAPTURE_WIDTH: u32 = 1600;
pub const CAPTURE_HEIGHT: u32 = 1200;

/// Fail before opening a window if the output cannot be created in an
/// existing directory.
pub fn check_output_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        return Err(format!("--screenshot path '{}' is a directory", path.display()));
    }
    if path.file_name().is_none() {
        return Err(format!("--screenshot path '{}' has no file name", path.display()));
    }
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    if !directory.is_dir() {
        return Err(format!(
            "--screenshot directory '{}' does not exist",
            directory.display()
        ));
    }
    Ok(())
}

/// Encode an RGBA8 image as a PNG into `writer`.
pub fn encode_png<W: Write>(image: &CapturedImage, writer: W) -> Result<(), String> {
    let expected = image.width as usize * image.height as usize * 4;
    if image.rgba.len() != expected {
        return Err(format!(
            "captured image has {} bytes, expected {expected} for {}x{}",
            image.rgba.len(), image.width, image.height
        ));
    }
    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer.write_image_data(&image.rgba).map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())
}

/// Render one frame, write it to `path`, and return its dimensions.
pub fn capture_to_png(renderer: &mut SceneRenderer, path: &Path) -> Result<(u32, u32), String> {
    let image = renderer
        .capture_rgba8(CAPTURE_WIDTH, CAPTURE_HEIGHT)
        .map_err(|error| format!("could not capture '{}': {error}", path.display()))?;
    let file = File::create(path)
        .map_err(|error| format!("could not create '{}': {error}", path.display()))?;
    let mut buffered = BufWriter::new(file);
    encode_png(&image, &mut buffered)
        .map_err(|error| format!("could not encode '{}': {error}", path.display()))?;
    buffered
        .flush()
        .map_err(|error| format!("could not write '{}': {error}", path.display()))?;
    Ok((image.width, image.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32) -> CapturedImage {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[(x * 40) as u8, (y * 60) as u8, 7, 255]);
            }
        }
        CapturedImage { width, height, rgba }
    }

    #[test]
    fn png_round_trips_pixels_and_size() {
        let source = image(5, 3);
        let mut bytes = Vec::new();
        encode_png(&source, &mut bytes).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).unwrap();
        assert_eq!((info.width, info.height), (5, 3));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(&buffer[..info.buffer_size()], source.rgba.as_slice());
    }

    #[test]
    fn mismatched_buffer_is_rejected() {
        let mut broken = image(2, 2);
        broken.rgba.pop();
        assert!(encode_png(&broken, Vec::new()).unwrap_err().contains("expected"));
    }

    #[test]
    fn output_path_checks() {
        let directory = std::env::temp_dir();
        assert!(check_output_path(&directory.join("frame.png")).is_ok());
        assert!(check_output_path(Path::new("frame.png")).is_ok());
        assert!(check_output_path(&directory).unwrap_err().contains("directory"));
        let missing = directory.join("svf-no-such-directory-5b").join("frame.png");
        assert!(check_output_path(&missing).unwrap_err().contains("does not exist"));
    }
}
