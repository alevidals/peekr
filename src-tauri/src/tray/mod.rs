mod badge;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use crate::popover;

const TRAY_ID: &str = "main";

/// The tray icon's current rectangle, used to place the popover when it is opened without a
/// click event (a second instance launching). Linux reports no rectangle.
pub(crate) fn icon_rect(app: &AppHandle) -> Option<tauri::Rect> {
    app.tray_by_id(TRAY_ID)?.rect().ok().flatten()
}

pub fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    // Linux tray icons (libappindicator) do not emit click events, so the menu is the only
    // way to open the popover there. Keeping the item on every platform also gives the
    // right-click menu a way to bring the popover back.
    let open = MenuItem::with_id(app, "open", "Open peekr", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit peekr", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    // On macOS the image is a template, so macOS tints the eye from the menu bar background.
    #[cfg(target_os = "macos")]
    let icon = badge::template_icon()?;
    #[cfg(not(target_os = "macos"))]
    let icon = badge::plain_icon()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(if badge::DEV { "peekr (dev)" } else { "peekr" })
        .on_menu_event(|app, event| {
            if event.id() == "open" {
                popover::show(app);
            } else if event.id() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                rect,
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                popover::toggle(tray.app_handle(), Some(rect));
            }
        })
        .build(app)?;

    // Draw the dev marker / empty state immediately, without waiting for the first poll.
    update_badge(app.handle(), 0);

    Ok(())
}

/// Updates the badge (`count > 0`) and the dev marker.
pub fn update_badge(app: &AppHandle, count: u32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    #[cfg(target_os = "macos")]
    update_overlay(&tray, count);

    #[cfg(not(target_os = "macos"))]
    {
        if count == 0 {
            if let Ok(icon) = badge::plain_icon() {
                let _ = tray.set_icon(Some(icon));
            }
            let _ = tray.set_icon_as_template(!badge::DEV);
        } else {
            let _ = tray.set_icon(Some(badge::badge_icon()));
            let _ = tray.set_icon_as_template(false);
        }
    }

    let tooltip = match count {
        0 if badge::DEV => "peekr (dev)".to_string(),
        0 => "peekr".to_string(),
        1 => "1 PR to review".to_string(),
        n => format!("{n} PRs to review"),
    };
    let _ = tray.set_tooltip(Some(&tooltip));
}

/// Draws the red badge / orange dev marker as layers on the status item button, on top of the
/// template image. Layers do not receive clicks, so the tray still behaves like a button.
#[cfg(target_os = "macos")]
fn update_overlay(tray: &tauri::tray::TrayIcon, count: u32) {
    use objc2::MainThreadMarker;

    let _ = tray.with_inner_tray_icon(move |inner| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(item) = inner.ns_status_item() else {
            return;
        };
        let Some(button) = item.button(mtm) else {
            return;
        };

        button.setWantsLayer(true);
        let Some(layer) = button.layer() else {
            return;
        };

        // Replace the overlays from the previous update. Collect first: removing a sublayer
        // mutates the array being enumerated.
        if let Some(sublayers) = unsafe { layer.sublayers() } {
            let previous: Vec<_> = sublayers
                .iter()
                .filter(|sublayer| {
                    sublayer
                        .name()
                        .is_some_and(|name| name.to_string().starts_with("peekr-"))
                })
                .collect();
            for sublayer in previous {
                sublayer.removeFromSuperlayer();
            }
        }

        let bounds = button.bounds();
        let flipped = button.isFlipped();
        let mid = (
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );

        if count > 0 {
            add_dot(
                layer.as_ref(),
                "peekr-badge",
                (1.0, 0.23, 0.19),
                7.0,
                true,
                mid,
                flipped,
            );
        }
        if badge::DEV {
            add_dot(
                layer.as_ref(),
                "peekr-dev",
                (1.0, 0.58, 0.0),
                5.0,
                false,
                mid,
                flipped,
            );
        }
    });
}

/// `top` places the dot in the icon's top-right corner, otherwise bottom-right. `mid` is the
/// status item button's centre in view coordinates.
#[cfg(target_os = "macos")]
fn add_dot(
    layer: &objc2_quartz_core::CALayer,
    name: &str,
    (r, g, b): (f64, f64, f64),
    diameter: f64,
    top: bool,
    mid: (f64, f64),
    flipped: bool,
) {
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::CGColor;
    use objc2_foundation::NSString;
    use objc2_quartz_core::CALayer;

    // tray-icon draws the status item image 18pt tall, centred in the button.
    const ICON_PT: f64 = 18.0;
    const INSET: f64 = 1.0;

    let dot = CALayer::new();
    dot.setName(Some(&NSString::from_str(name)));
    dot.setBackgroundColor(Some(&CGColor::new_srgb(r, g, b, 1.0)));
    dot.setCornerRadius(diameter / 2.0);

    let half = ICON_PT / 2.0;
    let x = mid.0 + half - diameter - INSET;
    let y = if top == !flipped {
        mid.1 + half - diameter - INSET
    } else {
        mid.1 - half + INSET
    };
    dot.setFrame(CGRect::new(
        CGPoint::new(x, y),
        CGSize::new(diameter, diameter),
    ));

    layer.addSublayer(&dot);
}
