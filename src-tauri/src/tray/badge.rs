use image::Rgba;
use imageproc::drawing::draw_filled_circle_mut;

const BASE_ICON: &[u8] = include_bytes!("../../icons/tray-icon.png");
const ICON_SIZE: u32 = 44;

/// True in debug builds (`tauri dev`): the tray icon gets a dev marker and can't be a template.
pub const DEV: bool = cfg!(debug_assertions);

pub fn plain_icon() -> tauri::Result<tauri::image::Image<'static>> {
    if DEV {
        return Ok(tauri::image::Image::new_owned(render(false), ICON_SIZE, ICON_SIZE));
    }
    tauri::image::Image::from_bytes(BASE_ICON)
}

pub fn badge_icon() -> tauri::image::Image<'static> {
    tauri::image::Image::new_owned(render(true), ICON_SIZE, ICON_SIZE)
}

fn is_dark_mode() -> bool {
    #[cfg(target_os = "macos")]
    {
        use objc::{class, msg_send, sel, sel_impl};
        use std::ffi::CStr;
        unsafe {
            let defaults: *mut objc::runtime::Object =
                msg_send![class!(NSUserDefaults), standardUserDefaults];
            let key = b"AppleInterfaceStyle\0";
            let ns_key: *mut objc::runtime::Object = msg_send![
                class!(NSString),
                stringWithUTF8String: key.as_ptr() as *const i8
            ];
            let value: *mut objc::runtime::Object = msg_send![defaults, stringForKey: ns_key];
            if value.is_null() {
                return false;
            }
            let c_str: *const i8 = msg_send![value, UTF8String];
            if c_str.is_null() {
                return false;
            }
            CStr::from_ptr(c_str).to_str().unwrap_or("") == "Dark"
        }
    }
    #[cfg(not(target_os = "macos"))]
    false
}

fn render(badge: bool) -> Vec<u8> {
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

    // Recolor the icon to match the menu bar: white in dark mode, black in light mode
    let icon_rgb: [u8; 3] = if is_dark_mode() { [255, 255, 255] } else { [0, 0, 0] };
    for pixel in img.pixels_mut() {
        if pixel[3] > 0 {
            pixel[0] = icon_rgb[0];
            pixel[1] = icon_rgb[1];
            pixel[2] = icon_rgb[2];
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
