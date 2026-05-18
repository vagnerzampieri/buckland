//! Embedded SVG assets for the tray icon. Assets are bundled at build
//! time via `include_bytes!` so the binary does not depend on a writable
//! resources/ directory at runtime.
//!
//! These constants are also used at startup by `tray::runtime` to install
//! the icons under `~/.local/share/icons/hicolor/scalable/apps/` so the
//! StatusNotifierItem host can resolve them by theme name.

pub const TRAY_IDLE_SVG: &[u8] = include_bytes!("../../resources/tray-idle.svg");
pub const TRAY_RUNNING_SVG: &[u8] = include_bytes!("../../resources/tray-running.svg");
pub const TRAY_ERROR_SVG: &[u8] = include_bytes!("../../resources/tray-error.svg");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_IDLE_SVG.is_empty());
        assert!(TRAY_IDLE_SVG.starts_with(b"<?xml") || TRAY_IDLE_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn running_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_RUNNING_SVG.is_empty());
        assert!(TRAY_RUNNING_SVG.starts_with(b"<?xml") || TRAY_RUNNING_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn error_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!TRAY_ERROR_SVG.is_empty());
        assert!(TRAY_ERROR_SVG.starts_with(b"<?xml") || TRAY_ERROR_SVG.starts_with(b"<svg"));
    }

    /// Regression: SNI hosts (gnome-shell + AppIndicator, snixembed, etc.)
    /// rasterize tray SVGs via librsvg with no CSS context. `currentColor`
    /// has no inherited `color` to resolve against, so it falls back to
    /// black — which renders nearly invisibly on Ubuntu's dark top panel.
    /// Tray SVGs must use explicit color values, never `currentColor`.
    #[test]
    fn tray_svgs_use_explicit_colors_not_currentcolor() {
        for (name, bytes) in [
            ("tray-idle.svg", TRAY_IDLE_SVG),
            ("tray-running.svg", TRAY_RUNNING_SVG),
            ("tray-error.svg", TRAY_ERROR_SVG),
        ] {
            let s = std::str::from_utf8(bytes).expect("svg is utf-8");
            assert!(
                !s.contains("currentColor"),
                "{name} uses `currentColor`, which librsvg resolves to black \
                 on dark tray panels — use an explicit color (e.g. #9E9E9E or #27AE60)"
            );
        }
    }
}
