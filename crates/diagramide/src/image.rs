use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    sync::Arc,
};

use eframe::egui::{self, ColorImage};
use resvg::usvg::{self, fontdb};

use crate::SPACE_MONO_BYTES;

const RENDER_WIDTH: f32 = 512.0;
const RENDER_LIMIT: f32 = 8192.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderBackground {
    Transparent,
    Color(egui::Color32),
}

pub fn write_png(file: String, image: ColorImage) -> Result<(), Box<dyn Error>> {
    let width = image.width() as u32;
    let height = image.height() as u32;

    let file = File::create(file)?;
    let w = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    let pixels: &[u8] = bytemuck::cast_slice(&image.pixels);
    writer.write_image_data(pixels)?;
    Ok(())
}
pub fn write_svg(file: String, svg: String) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(file)?;
    let bytes = svg.into_bytes();
    file.write_all(&bytes)?;
    Ok(())
}

pub fn render_svg_to_image(
    svg_content: &str,
    scale: f32,
    background: RenderBackground,
) -> Option<egui::ColorImage> {
    let mut db = fontdb::Database::new();
    db.load_font_data(SPACE_MONO_BYTES.to_vec());

    // 2. Parse the SVG
    let xml_opt = usvg::Options {
        fontdb: Arc::new(db),
        ..Default::default()
    };
    let tree: usvg::Tree = usvg::Tree::from_str(svg_content, &xml_opt).ok()?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());

    // 1. Determine base multiplier to target RENDER_WIDTH
    let base_mult = RENDER_WIDTH / w;

    // 2. Calculate final dimensions with scale, clamped to hardware limit
    let final_width_f = (w * base_mult * scale).min(RENDER_LIMIT);
    let final_height_f = (h * base_mult * scale).min(RENDER_LIMIT);

    // 3. Recalculate effective scale to ensure transform matches clamped pixel dimensions
    let effective_scale_x = final_width_f / w;
    let effective_scale_y = final_height_f / h;

    let width = final_width_f.ceil() as u32;
    let height = final_height_f.ceil() as u32;

    if width == 0 || height == 0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    if let RenderBackground::Color(color) = background {
        let [r, g, b, a] = color.to_srgba_unmultiplied();
        pixmap.fill(resvg::tiny_skia::Color::from_rgba8(r, g, b, a));
    }

    // Use the effective scale to ensure the SVG content fills the clamped pixmap exactly
    let transform = tiny_skia::Transform::from_scale(effective_scale_x, effective_scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(egui::ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
        <rect x="5" y="5" width="1" height="1" fill="#ff0000"/>
    </svg>"##;

    #[test]
    fn rasterizer_uses_explicit_background_color() {
        let background = egui::Color32::from_rgb(12, 34, 56);
        let image = render_svg_to_image(SVG, 1.0, RenderBackground::Color(background)).unwrap();
        assert_eq!(image.pixels[0], background);
    }

    #[test]
    fn rasterizer_keeps_transparent_background() {
        let image = render_svg_to_image(SVG, 1.0, RenderBackground::Transparent).unwrap();
        assert_eq!(image.pixels[0].a(), 0);
    }
}
