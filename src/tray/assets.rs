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
pub const APP_ICON_SVG: &[u8] = include_bytes!("../../resources/buckland.svg");

/// The freedesktop icon-theme names we expose. Hosts resolve these
/// against `~/.local/share/icons/hicolor/scalable/apps/<name>.svg`.
pub const ICON_NAME_IDLE: &str = "buckland-tray-idle";
pub const ICON_NAME_RUNNING: &str = "buckland-tray-running";
pub const ICON_NAME_ERROR: &str = "buckland-tray-error";

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

    #[test]
    fn app_icon_svg_is_non_empty_and_starts_with_svg_tag() {
        assert!(!APP_ICON_SVG.is_empty());
        assert!(APP_ICON_SVG.starts_with(b"<?xml") || APP_ICON_SVG.starts_with(b"<svg"));
    }

    #[test]
    fn icon_names_are_unique_and_kebab_cased() {
        let names = [ICON_NAME_IDLE, ICON_NAME_RUNNING, ICON_NAME_ERROR];
        let mut sorted = names.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 3, "icon names must be distinct");
        for n in &names {
            assert!(n.starts_with("buckland-"));
            assert!(n.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }
}
