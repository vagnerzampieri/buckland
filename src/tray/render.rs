//! Rasterize the embedded SVG icons into ARGB32 byte buffers so they can
//! be shipped to SNI hosts via `IconPixmap` (instead of `IconName`).
//!
//! Why pixmap instead of name: GNOME's `St.Icon` + AppIndicator pipeline
//! treats theme-named icons as symbolic masks and recolors them to the
//! panel's CSS `color`, which on Ubuntu's dark panel collapses our
//! `#FFFFFF` to ~`#303030` — the icon disappears visually. Pixmap bytes
//! ride D-Bus untouched and the host renders them pixel-for-pixel.

use crate::tray::assets;
use ksni::Icon;

/// Sizes we emit. The host picks the closest match for its panel.
pub(crate) const SIZES: &[u32] = &[16, 22, 32, 48];

/// Pre-rendered ARGB32 pixmaps for every tray state. Rendered once at
/// startup and cloned on each `icon_pixmap()` call.
#[derive(Clone)]
pub struct StatePixmaps {
    pub idle: Vec<Icon>,
    pub running: Vec<Icon>,
    pub error: Vec<Icon>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("svg parse failed: {0}")]
    Parse(String),
    #[error("allocate pixmap failed (size {0}x{0})")]
    Alloc(u32),
}

/// Rasterize the three embedded tray SVGs at all `SIZES`.
pub fn render_state_icons() -> Result<StatePixmaps, RenderError> {
    Ok(StatePixmaps {
        idle: render_sizes(assets::TRAY_IDLE_SVG)?,
        running: render_sizes(assets::TRAY_RUNNING_SVG)?,
        error: render_sizes(assets::TRAY_ERROR_SVG)?,
    })
}

fn render_sizes(svg_bytes: &[u8]) -> Result<Vec<Icon>, RenderError> {
    let tree = resvg::usvg::Tree::from_data(svg_bytes, &resvg::usvg::Options::default())
        .map_err(|e| RenderError::Parse(e.to_string()))?;
    SIZES.iter().map(|s| render_one(&tree, *s)).collect()
}

fn render_one(tree: &resvg::usvg::Tree, size: u32) -> Result<Icon, RenderError> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).ok_or(RenderError::Alloc(size))?;

    let svg_size = tree.size();
    let scale = size as f32 / svg_size.width().max(svg_size.height());
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);

    resvg::render(tree, transform, &mut pixmap.as_mut());

    // tiny_skia gives us premultiplied RGBA, little-endian byte order
    // (R, G, B, A). The SNI spec wants ARGB32 in network byte order
    // (A, R, G, B). For our fully-opaque SVGs, premultiplied == straight,
    // so the only work left is to permute the bytes.
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for px in pixmap.pixels() {
        data.push(px.alpha());
        data.push(px.red());
        data.push(px.green());
        data.push(px.blue());
    }

    Ok(Icon {
        width: size as i32,
        height: size as i32,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_idle_pixmap_at_22px_contains_pure_white_argb_pixels() {
        let pixmaps = render_state_icons().expect("render");
        let p22 = pixmaps
            .idle
            .iter()
            .find(|i| i.width == 22 && i.height == 22)
            .expect("22px idle pixmap");

        assert_eq!(
            p22.data.len(),
            (p22.width * p22.height * 4) as usize,
            "ARGB32 = 4 bytes per pixel",
        );

        // The idle SVG fill is #FFFFFF. Bytes must arrive as ARGB
        // big-endian (A, R, G, B) — at least one fully-opaque pure-white
        // pixel must exist. Anything else means we're upstream of the
        // bug we set out to fix.
        let has_pure_white = p22.data.chunks_exact(4).any(|px| {
            let (a, r, g, b) = (px[0], px[1], px[2], px[3]);
            a == 255 && r == 255 && g == 255 && b == 255
        });
        assert!(
            has_pure_white,
            "idle pixmap must contain at least one #FFFFFF/A=255 pixel; \
             otherwise the host sees dim/transparent bytes"
        );
    }

    #[test]
    fn render_state_icons_emits_one_pixmap_per_size_per_state() {
        let p = render_state_icons().expect("render");
        for v in [&p.idle, &p.running, &p.error] {
            assert_eq!(v.len(), SIZES.len(), "one pixmap per declared size",);
            for icon in v {
                assert!(SIZES.contains(&(icon.width as u32)));
                assert_eq!(icon.width, icon.height, "square pixmap");
            }
        }
    }

    #[test]
    fn render_running_pixmap_contains_pure_green_argb_pixels() {
        let p = render_state_icons().expect("render");
        let p22 = p
            .running
            .iter()
            .find(|i| i.width == 22)
            .expect("22px running");
        // Running SVG fill is #27AE60. The exact hex won't survive
        // anti-aliasing at every pixel, but the interior of the
        // central dot is fully covered — there must be a fully-opaque
        // pixel whose green channel dominates red and blue.
        let has_green_dominant = p22.data.chunks_exact(4).any(|px| {
            let (a, r, g, b) = (px[0], px[1], px[2], px[3]);
            a == 255 && g > r + 40 && g > b + 40
        });
        assert!(
            has_green_dominant,
            "running pixmap must contain unmistakable green pixels (G > R+40 and G > B+40)"
        );
    }
}
