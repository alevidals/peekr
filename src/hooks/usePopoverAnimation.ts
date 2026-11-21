import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onPopoverWillHide, onPopoverWillShow } from "../api";
import type { PopoverPlacement } from "../types";

/**
 * Drives the open/close animation: `animKey` changes each time the popover is shown (to
 * restart the CSS animation), `closing` is set when the backend announces it will hide, and
 * `placement` is the side of the tray icon the window opened on, so the animation grows away
 * from the icon instead of always downwards. `onOpen` runs on every show.
 */
export function usePopoverAnimation(onOpen?: () => void) {
  const [animKey, setAnimKey] = useState(0);
  const [closing, setClosing] = useState(false);
  const [placement, setPlacement] = useState<PopoverPlacement>("below");
  // Kept in a ref so the listeners subscribe once even if `onOpen` is unstable.
  const onOpenRef = useRef(onOpen);

  useEffect(() => {
    onOpenRef.current = onOpen;
  }, [onOpen]);

  useEffect(() => {
    const unlisten = onPopoverWillShow((nextPlacement) => {
      // Explicit show notification: focus may never arrive here (another Space,
      // full-screen app), so the closing state cannot depend on it.
      setClosing(false);
      setPlacement(nextPlacement);
      setAnimKey((k) => k + 1);
      onOpenRef.current?.();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // A click inside the window also clears the closing state, e.g. if the window was
  // focused without going through `popover-will-show`.
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) setClosing(false);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = onPopoverWillHide(() => setClosing(true));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { animKey, closing, placement };
}
