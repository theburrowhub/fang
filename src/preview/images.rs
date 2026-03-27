//! Rendering Mermaid diagrams and image files to PNG bytes for the preview panel.

/// Render a Mermaid diagram source to PNG bytes via mermaid-rs-renderer + resvg.
///
/// Returns `None` if rendering fails (the caller falls back to a code block).
pub fn render_mermaid_to_png(source: &str) -> Option<Vec<u8>> {
    // Step 1: Mermaid source → SVG string
    let svg = mermaid_rs_renderer::render(source)
        .map_err(|e| tracing::warn!("mermaid render failed: {}", e))
        .ok()?;

    svg_to_png(&svg)
}

/// Convert an SVG string to PNG bytes using resvg.
pub fn svg_to_png(svg: &str) -> Option<Vec<u8>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|e| tracing::warn!("usvg parse failed: {}", e))
        .ok()?;

    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(&tree, usvg::Transform::default(), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| tracing::warn!("PNG encode failed: {}", e))
        .ok()
}

/// Load an image file (PNG, JPEG, GIF, WebP) and return PNG bytes.
///
/// Normalises to PNG so we have a single format to decode in the renderer.
/// Returns `None` if the file cannot be loaded or decoded.
pub fn load_image_to_png(path: &std::path::Path) -> Option<Vec<u8>> {
    let img = image::open(path)
        .map_err(|e| tracing::debug!("image load failed {:?}: {}", path, e))
        .ok()?;

    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| tracing::warn!("PNG re-encode failed: {}", e))
        .ok()?;
    Some(buf)
}

/// Decode PNG bytes to a `DynamicImage` for ratatui-image.
pub fn png_to_dynamic_image(png: &[u8]) -> Option<image::DynamicImage> {
    image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .map_err(|e| tracing::warn!("DynamicImage decode failed: {}", e))
        .ok()
}
