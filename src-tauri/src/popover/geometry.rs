//! Pure screen geometry for the tray popover.
//!
//! `tauri-plugin-positioner` only models "above/below the icon" and measures against the
//! monitor instead of its work area, so a bottom taskbar leaves the window off-screen.
//! Here we take the icon rectangle reported by the tray event, find the screen edge it is
//! docked to, open towards the inside of the screen and clamp the result to the work area.
//!
//! Every value is in physical pixels, the unit tray events and monitor APIs use.

use serde::Serialize;

/// Space kept between the popover and the edges it does not open from, in logical pixels.
/// The edge facing the tray is flush with the bar, so the window looks attached to it.
const MARGIN: f64 = 8.0;
/// Insets smaller than this are rounding noise, not a taskbar, menu bar or dock.
const EDGE_EPSILON: i32 = 1;
/// Tolerance when comparing normalized distances.
const DISTANCE_EPSILON: f64 = 1e-6;

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn right(&self) -> i32 {
        self.x + self.width
    }

    fn bottom(&self) -> i32 {
        self.y + self.height
    }
}

/// Where the popover sits relative to the tray icon. Drives the entrance/exit animation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Above,
    #[default]
    Below,
    Left,
    Right,
}

/// The screen edge the tray icon is docked to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Places `window` next to `icon`, opening towards the inside of `work_area`.
///
/// Returns the top-left corner to move the window to, plus the side it ended up on.
pub fn place(
    icon: Rect,
    window: Rect,
    monitor: Rect,
    work_area: Rect,
    scale_factor: f64,
) -> (Rect, Placement) {
    let margin = (MARGIN * scale_factor).round() as i32;
    let centred_x = icon.x + icon.width / 2 - window.width / 2;
    let centred_y = icon.y + icon.height / 2 - window.height / 2;

    let (x, y, placement) = match dock_edge(icon, monitor, work_area) {
        // The icon is on the top edge: the popover opens below it, flush with the bar.
        Edge::Top => (centred_x, icon.bottom().max(work_area.y), Placement::Below),
        // Bottom edge: open above, ending flush with the work area (not inside the taskbar).
        Edge::Bottom => (
            centred_x,
            icon.y.min(work_area.bottom()) - window.height,
            Placement::Above,
        ),
        // Left edge (vertical taskbar): open to the right, flush with the bar.
        Edge::Left => (icon.right().max(work_area.x), centred_y, Placement::Right),
        // Right edge: open to the left, flush with the bar.
        Edge::Right => (
            icon.x.min(work_area.right()) - window.width,
            centred_y,
            Placement::Left,
        ),
    };

    // The anchored edge stays flush with the tray or menu bar, so the window looks attached.
    // The perpendicular axis is clamped with a margin: it keeps the rounded corners away
    // from the screen edges and the window on screen on multi-monitor setups with negative
    // coordinates or when the popover is larger than the available space.
    let (x, y) = match placement {
        Placement::Below | Placement::Above => (
            clamp(
                x,
                work_area.x + margin,
                work_area.right() - margin - window.width,
            ),
            clamp(y, work_area.y, work_area.bottom() - window.height),
        ),
        Placement::Left | Placement::Right => (
            clamp(x, work_area.x, work_area.right() - window.width),
            clamp(
                y,
                work_area.y + margin,
                work_area.bottom() - margin - window.height,
            ),
        ),
    };

    (Rect::new(x, y, window.width, window.height), placement)
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if max < min {
        min
    } else {
        value.clamp(min, max)
    }
}

/// Picks the screen edge the icon belongs to.
///
/// The work area tells us which edges have a taskbar, panel or dock reserved; the icon's own
/// position breaks ties (macOS reserves both the menu bar and the Dock, for example, but the
/// icon lives on the menu bar).
fn dock_edge(icon: Rect, monitor: Rect, work_area: Rect) -> Edge {
    let is_reserved = |edge| match edge {
        Edge::Top => work_area.y - monitor.y > EDGE_EPSILON,
        Edge::Bottom => monitor.bottom() - work_area.bottom() > EDGE_EPSILON,
        Edge::Left => work_area.x - monitor.x > EDGE_EPSILON,
        Edge::Right => monitor.right() - work_area.right() > EDGE_EPSILON,
    };

    let mut best = Edge::Top;
    let mut best_distance = f64::INFINITY;
    let mut best_reserved = false;

    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        let distance = edge_distance(edge, icon, work_area);
        let reserved = is_reserved(edge);
        let closer = distance < best_distance - DISTANCE_EPSILON;
        // Same distance: prefer an edge the OS actually reserved for a bar.
        let same_but_reserved =
            (distance - best_distance).abs() <= DISTANCE_EPSILON && reserved && !best_reserved;
        if closer || same_but_reserved {
            best = edge;
            best_distance = distance;
            best_reserved = reserved;
        }
    }

    best
}

/// Distance from the icon to an edge of the work area, normalized by the work-area size so
/// horizontal and vertical distances can be compared. An icon outside the work area (in a
/// taskbar or the menu bar) counts as touching it.
fn edge_distance(edge: Edge, icon: Rect, work_area: Rect) -> f64 {
    let (distance, size) = match edge {
        Edge::Top => (icon.y - work_area.y, work_area.height),
        Edge::Bottom => (work_area.bottom() - icon.bottom(), work_area.height),
        Edge::Left => (icon.x - work_area.x, work_area.width),
        Edge::Right => (work_area.right() - icon.right(), work_area.width),
    };
    distance.max(0) as f64 / size.max(1) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect {
        x: 0,
        y: 0,
        width: 380,
        height: 500,
    };

    /// A 1920x1080 monitor with a 48 px taskbar on the given edge.
    fn screen_with_bar(edge: Edge) -> (Rect, Rect) {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let work = match edge {
            Edge::Top => Rect::new(0, 48, 1920, 1032),
            Edge::Bottom => Rect::new(0, 0, 1920, 1032),
            Edge::Left => Rect::new(48, 0, 1872, 1080),
            Edge::Right => Rect::new(0, 0, 1872, 1080),
        };
        (monitor, work)
    }

    fn assert_inside(window: Rect, work: Rect) {
        assert!(window.x >= work.x, "window {window:?} too far left");
        assert!(window.y >= work.y, "window {window:?} too high");
        assert!(
            window.right() <= work.right(),
            "window {window:?} too far right"
        );
        assert!(
            window.bottom() <= work.bottom(),
            "window {window:?} too low"
        );
    }

    #[test]
    fn bottom_taskbar_opens_above_flush_with_the_taskbar() {
        let (monitor, work) = screen_with_bar(Edge::Bottom);
        let icon = Rect::new(1850, 1040, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Above);
        assert_eq!(window.bottom(), work.bottom());
        assert_inside(window, work);
    }

    #[test]
    fn top_taskbar_opens_below_flush_with_the_taskbar() {
        let (monitor, work) = screen_with_bar(Edge::Top);
        let icon = Rect::new(1850, 12, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Below);
        assert_eq!(window.y, work.y);
        assert_inside(window, work);
    }

    #[test]
    fn left_taskbar_opens_to_the_right_flush_with_the_taskbar() {
        let (monitor, work) = screen_with_bar(Edge::Left);
        let icon = Rect::new(8, 1040, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Right);
        assert_eq!(window.x, work.x);
        assert_inside(window, work);
    }

    #[test]
    fn right_taskbar_opens_to_the_left_flush_with_the_taskbar() {
        let (monitor, work) = screen_with_bar(Edge::Right);
        let icon = Rect::new(1876, 1040, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Left);
        assert_eq!(window.right(), work.right());
        assert_inside(window, work);
    }

    #[test]
    fn macos_reserves_menu_bar_and_dock_but_opens_below_the_icon() {
        // visibleFrame: menu bar 25 px on top, Dock 70 px at the bottom.
        let monitor = Rect::new(0, 0, 1440, 900);
        let work = Rect::new(0, 25, 1440, 805);
        let icon = Rect::new(1200, 0, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Below);
        // Flush against the bottom of the menu bar, not floating below it.
        assert_eq!(window.y, work.y);
        assert_inside(window, work);
    }

    #[test]
    fn without_reserved_insets_the_nearest_edge_wins() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let icon = Rect::new(1850, 1050, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, monitor, 1.0);

        assert_eq!(placement, Placement::Above);
        // No taskbar to attach to: the window sits directly above the icon.
        assert_eq!(window.bottom(), icon.y);
        assert_inside(window, monitor);
    }

    #[test]
    fn centres_on_the_icon_until_the_work_area_edge() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let work = Rect::new(0, 0, 1920, 1032);
        let icon = Rect::new(900, 1040, 24, 24);

        let (window, _) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(window.x, icon.x + icon.width / 2 - window.width / 2);
    }

    #[test]
    fn clamps_a_wide_popover_at_the_right_edge() {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let work = Rect::new(0, 0, 1920, 1032);
        let icon = Rect::new(1890, 1040, 24, 24);

        let (window, _) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(window.x + window.width, work.right() - MARGIN as i32);
        assert_inside(window, work);
    }

    #[test]
    fn negative_monitor_origin_is_respected() {
        // Secondary monitor placed to the left of the primary one.
        let monitor = Rect::new(-1920, 0, 1920, 1080);
        let work = Rect::new(-1920, 0, 1920, 1032);
        let icon = Rect::new(-100, 1040, 24, 24);

        let (window, placement) = place(icon, WINDOW, monitor, work, 1.0);

        assert_eq!(placement, Placement::Above);
        assert_inside(window, work);
    }

    #[test]
    fn scales_the_perpendicular_margin() {
        let monitor = Rect::new(0, 0, 3840, 2160);
        let work = Rect::new(0, 0, 3840, 2064);
        let icon = Rect::new(3800, 2080, 48, 48);

        let (window, placement) = place(icon, WINDOW, monitor, work, 2.0);

        assert_eq!(placement, Placement::Above);
        assert_eq!(window.bottom(), work.bottom());
        // 8 logical px of margin at 2x, keeping the window away from the right edge.
        assert_eq!(window.right(), work.right() - 16);
        assert_inside(window, work);
    }
}
