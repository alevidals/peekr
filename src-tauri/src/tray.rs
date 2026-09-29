use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use image::Rgba;
use imageproc::drawing::{draw_filled_circle_mut, draw_text_mut};
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_positioner::{Position, WindowExt};

const BASE_ICON: &[u8] = include_bytes!("../icons/tray-icon.png");
const BADGE_FONT: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
const ICON_SIZE: u32 = 44;

pub fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let icon = tauri::image::Image::from_bytes(BASE_ICON)?;

    let quit = MenuItem::with_id(app, "quit", "Quit peekr", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("peekr")
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("popover") {
                    if window.is_visible().unwrap_or(false) {
                        // Bump the generation counter regardless of whether the blur
                        // handler already started a close. This cancels any earlier task
                        // (blur-initiated) and starts a fresh one owned by this click,
                        // ensuring exactly one hide fires 150 ms from now.
                        //
                        // On macOS, Focused(false) fires at MouseDown so the blur handler
                        // will have already incremented the counter once; we increment it
                        // again here, making the blur task a no-op. On other platforms
                        // where blur may arrive after or not at all, this path still works.
                        let hs  = app.state::<crate::state::PopoverHideState>();
                        let gen = hs.close_gen.fetch_add(1, Ordering::SeqCst) + 1;
                        let _   = window.emit("popover-will-hide", ());
                        let w   = window.clone();
                        let cg  = hs.close_gen.clone();
                        let fa  = hs.focused_at.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(Duration::from_millis(150)).await;
                            if cg.load(Ordering::SeqCst) == gen {
                                let now2 = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                fa.store(now2, Ordering::SeqCst);
                                let _ = w.hide();
                            }
                        });
                    } else {
                        // Position first (before the window is visible).
                        let _ = window.move_window(Position::TrayBottomCenter);

                        #[cfg(target_os = "macos")]
                        if let Ok(raw_win) = window.ns_window() {
                            // Call activate + makeKeyAndOrderFront: synchronously in the
                            // same run-loop turn, bypassing Tauri's dispatch queue.
                            // window.set_focus() queues makeKeyAndOrderFront: for the
                            // next event-loop iteration; by then macOS has already
                            // re-assigned the active app, so the window never becomes key.
                            unsafe {
                                use objc::{class, msg_send, sel, sel_impl};
                                let null: *mut objc::runtime::Object = std::ptr::null_mut();
                                let ns_app: *mut objc::runtime::Object =
                                    msg_send![class!(NSApplication), sharedApplication];
                                let _: () = msg_send![ns_app, activateIgnoringOtherApps: true as objc::runtime::BOOL];
                                let ns_win = raw_win as *mut objc::runtime::Object;
                                let _: () = msg_send![ns_win, makeKeyAndOrderFront: null];
                            }
                        }

                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

pub fn update_tray_badge(app: &AppHandle, count: u32) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };

    if count == 0 {
        // Restore plain template icon
        if let Ok(icon) = tauri::image::Image::from_bytes(BASE_ICON) {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_icon_as_template(true);
        }
        let _ = tray.set_tooltip(Some("peekr"));
        return;
    }

    let rgba = render_badge(count);
    let icon =
        tauri::image::Image::new_owned(rgba, ICON_SIZE, ICON_SIZE);
    let _ = tray.set_icon(Some(icon));
    let _ = tray.set_icon_as_template(false);

    let tooltip = if count == 1 {
        "1 PR to review".to_string()
    } else {
        format!("{} PRs to review", count)
    };
    let _ = tray.set_tooltip(Some(&tooltip));
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

fn render_badge(count: u32) -> Vec<u8> {
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

    let label = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };

    // Badge circle in top-right corner
    let badge_r: i32 = 9;
    let cx = (ICON_SIZE as i32) - badge_r - 1;
    let cy = badge_r + 1;
    draw_filled_circle_mut(&mut img, (cx, cy), badge_r, Rgba([255u8, 59, 48, 255]));

    // Draw white text centred in badge
    if let Ok(font) = FontRef::try_from_slice(BADGE_FONT) {
        let scale = PxScale::from(10.0);
        let text_width = measure_text_width(&font, &label, scale);
        let text_x = cx - (text_width as i32 / 2);
        let text_y = cy - 6;
        draw_text_mut(
            &mut img,
            Rgba([255u8, 255, 255, 255]),
            text_x,
            text_y,
            scale,
            &font,
            &label,
        );
    }

    img.into_raw()
}

fn measure_text_width(font: &FontRef, text: &str, scale: PxScale) -> f32 {
    let scaled = font.as_scaled(scale);
    text.chars()
        .map(|c| {
            let glyph_id = font.glyph_id(c);
            scaled.h_advance(glyph_id)
        })
        .sum()
}
