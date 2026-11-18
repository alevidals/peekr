//! Tray icon drawing.
//!
//! On macOS the status item image is a template: macOS tints it from the actual menu bar
//! background, so it stays visible over any wallpaper, Space or full-screen app. The badge
//! and the dev marker are drawn as layers on top of the status item button instead of being
//! baked into the image (see `update_overlay` in the parent module).

const BASE_ICON: &[u8] = include_bytes!("../../icons/tray-icon.png");

/// True in debug builds (`tauri dev`): the tray icon gets a dev marker.
pub const DEV: bool = cfg!(debug_assertions);

#[cfg(target_os = "macos")]
pub fn template_icon() -> tauri::Result<tauri::image::Image<'static>> {
    tauri::image::Image::from_bytes(BASE_ICON)
}

#[cfg(not(target_os = "macos"))]
const ICON_SIZE: u32 = 44;

#[cfg(not(target_os = "macos"))]
pub fn plain_icon() -> tauri::Result<tauri::image::Image<'static>> {
    if DEV {
        return Ok(tauri::image::Image::new_owned(
            render(false),
            ICON_SIZE,
            ICON_SIZE,
        ));
    }
    tauri::image::Image::from_bytes(BASE_ICON)
}

#[cfg(not(target_os = "macos"))]
pub fn badge_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::new_owned(render(true), ICON_SIZE, ICON_SIZE)
}

#[cfg(not(target_os = "macos"))]
fn render(badge: bool) -> Vec<u8> {
    use image::Rgba;
    use imageproc::drawing::draw_filled_circle_mut;

    let base = image::load_from_memory_with_format(BASE_ICON, image::ImageFormat::Png)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(ICON_SIZE, ICON_SIZE));
    let mut img = base.to_rgba8();

    if img.width() != ICON_SIZE || img.height() != ICON_SIZE {
        img = image::imageops::resize(
            &img,
            ICON_SIZE,
            ICON_SIZE,
            image::imageops::FilterType::Lanczos3,
        );
    }

    for pixel in img.pixels_mut() {
        if pixel[3] > 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
        }
    }

    if DEV {
        // Orange dev marker in the bottom-right corner
        let r: i32 = 7;
        let c = (ICON_SIZE as i32) - r - 1;
        draw_filled_circle_mut(&mut img, (c, c), r, Rgba([255u8, 149, 0, 255]));
    }

    if !badge {
        return img.into_raw();
    }

    // Badge circle in top-right corner
    let badge_r: i32 = 9;
    let cx = (ICON_SIZE as i32) - badge_r - 1;
    let cy = badge_r + 1;
    draw_filled_circle_mut(&mut img, (cx, cy), badge_r, Rgba([255u8, 59, 48, 255]));

    img.into_raw()
}
