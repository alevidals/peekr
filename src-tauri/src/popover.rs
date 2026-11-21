//! Show/hide behaviour of the tray popover window.
//!
//! Closing is a two-step affair: the frontend is told to play its closing animation
//! (`popover-will-hide`), and 150 ms later the window is actually hidden. Blur events and
//! tray clicks both start that sequence, so the "who closes it" race is resolved with a
//! generation counter: every close attempt bumps it, and the delayed hide only runs if no
//! newer attempt (or focus gain) happened in the meantime.

mod geometry;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use geometry::Placement;
use tauri::{App, AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

#[cfg(target_os = "macos")]
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt};

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSWorkspace, NSWorkspaceActiveSpaceDidChangeNotification};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol};

// macOS tray popover. An `NSPanel` with the non-activating style can be ordered front and
// made key without activating the app, which is what lets it appear on the current Space,
// over a full-screen app, and still receive keyboard input. Kept in its own module because
// the macro injects `use` items into the surrounding scope.
#[cfg(target_os = "macos")]
mod panel_class {
    tauri_nspanel::tauri_panel! {
        panel!(PopoverPanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                is_floating_panel: true
            }
        })
    }
}

#[cfg(target_os = "macos")]
use panel_class::PopoverPanel;

#[cfg(target_os = "macos")]
struct SpaceObserverIvars {
    app: AppHandle,
}

// Hides the panel when the user switches to another desktop or enters/leaves full screen,
// instead of letting `canJoinAllSpaces` drag it along to the new Space.
#[cfg(target_os = "macos")]
define_class!(
    // SAFETY: NSObject subclass whose only ivar is a Send + Sync AppHandle.
    #[unsafe(super(NSObject))]
    #[name = "PeekrSpaceObserver"]
    #[ivars = SpaceObserverIvars]
    struct SpaceObserver;

    unsafe impl NSObjectProtocol for SpaceObserver {}

    impl SpaceObserver {
        #[unsafe(method(activeSpaceDidChange:))]
        fn active_space_did_change(&self, _notification: &NSNotification) {
            let app = &self.ivars().app;
            let Some(window) = app.get_webview_window(LABEL) else {
                return;
            };
            // Ignore the change that may accompany opening the panel itself.
            if window.is_visible().unwrap_or(false) && !app.state::<HideState>().is_spurious_blur() {
                hide_now(&window);
            }
        }
    }
);

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

/// Milliseconds since the first call. Monotonic, so a clock change cannot break the guards
/// that compare two of these timestamps.
fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
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
    #[cfg(target_os = "macos")]
    observe_space_changes(app.handle());

    let Some(popover) = app.get_webview_window(LABEL) else {
        return;
    };
    configure_panel(&popover);
    let app_handle = app.handle().clone();
    let window = popover.clone();

    popover.on_window_event(move |event| {
        let hide_state = app_handle.state::<HideState>();
        match event {
            WindowEvent::Focused(true) => hide_state.on_focus_gained(),
            // A click outside closes the popover.
            WindowEvent::Focused(false) if !hide_state.is_spurious_blur() => schedule_hide(&window),
            _ => {}
        }
    });
}

/// Hides immediately, skipping the exit animation (used when the user leaves the Space).
#[cfg(target_os = "macos")]
fn hide_now(window: &WebviewWindow) {
    let app = window.app_handle();
    let state = app.state::<HideState>();
    // Cancel any delayed hide and ignore the Focused(false) that hide() triggers.
    state.begin_close();
    state.mark_now();
    let _ = window.hide();
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

/// Tray-icon click: open the popover next to the icon, or close it if already visible.
///
/// `icon` is the tray icon rectangle from the click event. It tells us which screen edge the
/// icon (and therefore the taskbar or menu bar) is on, so the window always opens inwards.
pub fn toggle(app: &AppHandle, icon: Option<tauri::Rect>) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };

    // A window that is "visible" on another Space (or behind a full-screen app) is not
    // really in front of the user: bring it here instead of hiding it.
    if window.is_visible().unwrap_or(false) && is_on_active_space(&window) {
        // On macOS, Focused(false) fires at mouse-down so the blur handler has already started
        // a close; starting another one here supersedes it, leaving exactly one hide.
        schedule_hide(&window);
        return;
    }

    show_at(app, &window, icon);
}

/// Shows the popover without toggling it. Used when a second instance launches.
pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    show_at(app, &window, crate::tray::icon_rect(app));
}

/// Moves the window next to the icon, tells the UI which side it opened on, then shows it.
fn show_at(app: &AppHandle, window: &WebviewWindow, icon: Option<tauri::Rect>) {
    // Position before the window becomes visible.
    let placement = move_next_to_tray(window, icon).unwrap_or_default();
    // On another Space or over a full-screen app, macOS may not deliver Focused(true)
    // until the user clicks inside. Tell the UI to leave its closing state first, or the
    // window stays showing the exit animation's final frame (fully transparent).
    let _ = window.emit("popover-will-show", placement);
    // Fresh timestamp so the Space observer does not treat the show as a desktop switch.
    app.state::<HideState>().mark_now();
    show_focused(window);
}

/// Places the popover against the screen edge the tray icon is docked to.
///
/// Returns the side it opened on, for the entrance animation, or `None` when the icon
/// rectangle is unknown (Linux, where tray icons report no position): the window then keeps
/// its previous position, which is always on screen.
fn move_next_to_tray(window: &WebviewWindow, icon: Option<tauri::Rect>) -> Option<Placement> {
    let icon = icon?;
    let size = window.outer_size().ok()?;
    let position = icon.position.to_physical::<f64>(1.0);
    let icon_size = icon.size.to_physical::<u32>(1.0);

    // The tray icon can be on any monitor; look it up by the icon's own centre.
    let center = (
        position.x + icon_size.width as f64 / 2.0,
        position.y + icon_size.height as f64 / 2.0,
    );
    let monitor = window
        .monitor_from_point(center.0, center.1)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten())?;

    let monitor_position = *monitor.position();
    let monitor_size = *monitor.size();
    let work_area = monitor.work_area();

    let icon_rect = geometry::Rect::new(
        position.x.round() as i32,
        position.y.round() as i32,
        icon_size.width as i32,
        icon_size.height as i32,
    );
    let window_rect = geometry::Rect::new(0, 0, size.width as i32, size.height as i32);
    let monitor_rect = geometry::Rect::new(
        monitor_position.x,
        monitor_position.y,
        monitor_size.width as i32,
        monitor_size.height as i32,
    );
    let work_rect = geometry::Rect::new(
        work_area.position.x,
        work_area.position.y,
        work_area.size.width as i32,
        work_area.size.height as i32,
    );

    let (target, placement) = geometry::place(
        icon_rect,
        window_rect,
        monitor_rect,
        work_rect,
        monitor.scale_factor(),
    );
    if let Err(e) = window.set_position(PhysicalPosition::new(target.x, target.y)) {
        log::warn!("[popover] could not move the window next to the tray icon: {e}");
    }
    Some(placement)
}

/// Converts the configured window into a non-activating `NSPanel` and lets it join every
/// Space, including a full-screen one. Without this, macOS refuses to draw a normal window
/// over another app's full-screen Space and the popover ends up on the desktop instead.
#[cfg(target_os = "macos")]
fn configure_panel(window: &WebviewWindow) {
    let Ok(panel) = window.to_panel::<PopoverPanel>() else {
        log::warn!("[popover] could not convert the window to an NSPanel");
        return;
    };

    panel.set_level(PanelLevel::Floating.value());
    // Non-activating: can become key without activating the app (no Space switch).
    if let Err(e) = panel.add_style_mask(StyleMask::empty().nonactivating_panel().into()) {
        log::warn!("[popover] could not make the panel non-activating: {e}");
    }
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .full_screen_auxiliary()
            .move_to_active_space()
            .into(),
    );
    panel.set_hides_on_deactivate(false);
    panel.set_works_when_modal(true);
}

#[cfg(not(target_os = "macos"))]
fn configure_panel(_window: &WebviewWindow) {}

/// Registers the observers that hide the panel on desktop/full-screen changes and redraw the
/// tray icon when the menu bar appearance changes.
#[cfg(target_os = "macos")]
fn observe_space_changes(app: &AppHandle) {
    let observer = SpaceObserver::alloc().set_ivars(SpaceObserverIvars { app: app.clone() });
    let observer: Retained<SpaceObserver> = unsafe { msg_send![super(observer), init] };

    let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();
    unsafe {
        workspace_center.addObserver_selector_name_object(
            &*(Retained::as_ptr(&observer) as *const AnyObject),
            sel!(activeSpaceDidChange:),
            Some(NSWorkspaceActiveSpaceDidChangeNotification),
            None,
        );
    }
    // Notification centers do not retain their observers: keep it alive for the app lifetime.
    std::mem::forget(observer);
}

/// Whether the window is on the Space the user is currently looking at.
#[cfg(target_os = "macos")]
fn is_on_active_space(window: &WebviewWindow) -> bool {
    use objc2_app_kit::NSWindow;

    let Ok(raw_win) = window.ns_window() else {
        return true;
    };
    unsafe {
        let ns_win = &*(raw_win.cast::<NSWindow>());
        ns_win.isOnActiveSpace()
    }
}

#[cfg(not(target_os = "macos"))]
fn is_on_active_space(_window: &WebviewWindow) -> bool {
    true
}

#[cfg(target_os = "macos")]
fn show_focused(window: &WebviewWindow) {
    // Order front and make the panel key without activating the app: that is what keeps the
    // user on the current Space / full-screen app, and still routes the keyboard to the
    // webview.
    if let Ok(panel) = window.app_handle().get_webview_panel(LABEL) {
        panel.show_and_make_key();
        return;
    }
    let _ = window.show();
    let _ = window.set_focus();
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
