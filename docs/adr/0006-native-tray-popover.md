# ADR-0006: Native tray popover

- **Status:** Accepted
- **Date:** 2026-11-19

## Context

A tray app is only useful if the popover appears where the user is looking. On
macOS, a normal always-on-top window does not open over another app's full-screen
Space, and activating the app to show it switches Spaces. The app must also close
predictably on a click outside, on `Esc`, on a second tray click, and when the
user changes desktop.

Windows and Linux do not expose the same panel model, but they do have tray
positioning and ordinary always-on-top windows.

## Decision

1. **One window label, two behaviours.** The `popover` window is converted into
   an `NSPanel` on macOS and left as a positioned window elsewhere.
2. **macOS panel configuration:**
   - floating level;
   - `nonactivating_panel` style so it can become key without activating the app;
   - `full_screen_auxiliary` and `move_to_active_space`;
   - `hides_on_deactivate(false)`;
   - `works_when_modal(true)`.
3. **Position through the plugin.** `tauri-plugin-positioner` moves the window to
   `TrayBottomCenter` before it becomes visible.
4. **Space observer.** An `NSWorkspace` observer hides the panel when the user
   switches Space or enters/leaves full screen, instead of dragging it along.
5. **Two-step close.** Emit `popover-will-hide`, wait 150 ms for the exit
   animation, then hide. A generation counter resolves tray-click vs blur races;
   a focus gain cancels an in-flight close.
6. **Show handshake.** Emit `popover-will-show` before showing so the UI leaves its
   closing state. A fresh focus timestamp prevents the Space observer from
   treating the show as a desktop switch.
7. **Template tray image.** macOS tints the eye from the actual menu bar. The
   badge and dev marker are `CALayer` overlays so they keep their colours.
8. **Cross-platform badge.** Windows/Linux redraw a 44×44 PNG with
   `image`/`imageproc`, normalising the eye to black and drawing the dots.
9. **Settings is separate.** It is an ordinary decorated window, not part of the
   panel; opening it hides the popover and switches the activation policy to
   `Regular`.

## Consequences

**Positive**

- The popover opens over full-screen apps without a Space switch.
- Keyboard input works while the app stays in the background.
- Closing is deterministic across blur, tray clicks and `Esc`.
- The tray icon stays legible on any wallpaper and keeps its badge colours.

**Negative / accepted**

- The macOS path depends on private API and `tauri-nspanel`; the app is not
  sandboxed and cannot ship on the Mac App Store as-is.
- The `objc2` code is platform-specific and needs real-device testing.
- Windows/Linux get a simpler behaviour: the popover cannot float over another
  app's full-screen space.
- The 150 ms close window is a visual contract between Rust and CSS.

## Alternatives considered

- **A normal window with `alwaysOnTop`:** does not reliably appear on the active
  Space or over full-screen apps.
- **Activate the app before showing:** steals focus and switches Spaces, which is
  exactly what a tray utility should avoid.
- **Bake the badge into the image on macOS:** loses the template tint and the
  badge colours.
- **A separate native menu:** cannot render search, themes or grouped PR rows.

## Validation

Unit tests cover the hide-generation and spurious-blur logic. The macOS manual
pass covers full-screen, multiple Spaces, multiple displays, focus restoration and
sleep/wake. Windows/Linux placement is part of the release QA matrix.
