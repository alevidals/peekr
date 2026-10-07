mod badge;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use crate::popover;

const TRAY_ID: &str = "main";

pub fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let quit = MenuItem::with_id(app, "quit", "Quit peekr", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(badge::plain_icon()?)
        .icon_as_template(!badge::DEV)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(if badge::DEV { "peekr (dev)" } else { "peekr" })
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
                popover::toggle(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Shows `count` as a red badge on the tray icon (plain template icon when zero).
pub fn update_badge(app: &AppHandle, count: u32) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    if count == 0 {
        if let Ok(icon) = badge::plain_icon() {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_icon_as_template(!badge::DEV);
        }
        let _ = tray.set_tooltip(Some(if badge::DEV { "peekr (dev)" } else { "peekr" }));
        return;
    }

    let _ = tray.set_icon(Some(badge::badge_icon()));
    let _ = tray.set_icon_as_template(false);

    let tooltip = if count == 1 {
        "1 PR to review".to_string()
    } else {
        format!("{count} PRs to review")
    };
    let _ = tray.set_tooltip(Some(&tooltip));
}
