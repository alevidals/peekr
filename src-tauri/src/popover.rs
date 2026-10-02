//! Show/hide behaviour of the tray popover window.
//!
//! Closing is a two-step affair: the frontend is told to play its closing animation
//! (`popover-will-hide`), and 150 ms later the window is actually hidden. Blur events and
//! tray clicks both start that sequence, so the "who closes it" race is resolved with a
//! generation counter: every close attempt bumps it, and the delayed hide only runs if no
//! newer attempt (or focus gain) happened in the meantime.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{App, AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

pub const LABEL: &str = "popover";
const HIDE_DELAY: Duration = Duration::from_millis(150);
/// macOS fires a spurious Focused(false) right after a programmatic hide()/show();
/// blur events this soon after the last focus change are ignored.
const BLUR_GUARD_MS: u64 = 300;

#[derive(Default)]
pub struct HideState {
    close_gen: AtomicU64,
    /// ms since UNIX_EPOCH of the last focus gain or programmatic hide.
    focused_at: AtomicU64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl HideState {
    fn mark_now(&self) {
        self.focused_at.store(now_ms(), Ordering::SeqCst);
    }

    /// Cancels any in-flight hide.
    fn on_focus_gained(&self) {
        self.close_gen.fetch_add(1, Ordering::SeqCst);
        self.mark_now();
    }

    fn is_spurious_blur(&self) -> bool {
        now_ms().saturating_sub(self.focused_at.load(Ordering::SeqCst)) < BLUR_GUARD_MS
    }

    /// Starts a close attempt, superseding earlier ones, and returns its generation.
    fn begin_close(&self) -> u64 {
        self.close_gen.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_current(&self, generation: u64) -> bool {
        self.close_gen.load(Ordering::SeqCst) == generation
    }
}

/// Registers the hide state and the click-outside (blur) behaviour.
pub fn setup(app: &App) {
    app.manage(HideState::default());

    let Some(popover) = app.get_webview_window(LABEL) else {
        return;
    };
    let app_handle = app.handle().clone();
    let window = popover.clone();

    popover.on_window_event(move |event| {
        let hide_state = app_handle.state::<HideState>();
        match event {
            WindowEvent::Focused(true) => hide_state.on_focus_gained(),
            WindowEvent::Focused(false) if !hide_state.is_spurious_blur() => schedule_hide(&window),
            _ => {}
        }
    });
}

pub fn schedule_hide(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    let generation = app.state::<HideState>().begin_close();
    let _ = window.emit("popover-will-hide", ());

    let window = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(HIDE_DELAY).await;
        let hide_state = app.state::<HideState>();
        if hide_state.is_current(generation) {
            // Recorded before hide() so the Focused(false) it triggers hits the guard.
            hide_state.mark_now();
            let _ = window.hide();
        }
    });
}

/// Tray-icon click: open the popover under the icon, or close it if already visible.
pub fn toggle(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        // On macOS, Focused(false) fires at mouse-down so the blur handler has already started
        // a close; starting another one here supersedes it, leaving exactly one hide.
        schedule_hide(&window);
        return;
    }

    // Position before the window becomes visible.
    let _ = window.move_window(Position::TrayBottomCenter);
    show_focused(&window);
}

#[cfg(target_os = "macos")]
fn show_focused(window: &WebviewWindow) {
    // Call activate + makeKeyAndOrderFront: synchronously in the same run-loop turn,
    // bypassing Tauri's dispatch queue. window.set_focus() queues makeKeyAndOrderFront:
    // for the next event-loop iteration; by then macOS has already re-assigned the active
    // app, so the window never becomes key.
    if let Ok(raw_win) = window.ns_window() {
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
}

#[cfg(not(target_os = "macos"))]
fn show_focused(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_close_attempt_supersedes_older() {
        let state = HideState::default();
        let first = state.begin_close();
        let second = state.begin_close();
        assert!(!state.is_current(first));
        assert!(state.is_current(second));
    }

    #[test]
    fn focus_gain_cancels_pending_hide_and_arms_blur_guard() {
        let state = HideState::default();
        let pending = state.begin_close();
        state.on_focus_gained();
        assert!(!state.is_current(pending));
        assert!(state.is_spurious_blur());
    }
}
