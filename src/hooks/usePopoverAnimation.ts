import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onPopoverWillHide } from "../api";

/**
 * Drives the open/close animation: `animKey` changes each time the window gains focus (to
 * restart the CSS animation) and `closing` is set when the backend announces it will hide.
 * `onOpen` runs on every focus gain.
 */
export function usePopoverAnimation(onOpen?: () => void) {
  const [animKey, setAnimKey] = useState(0);
  const [closing, setClosing] = useState(false);

  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        setClosing(false);
        setAnimKey((k) => k + 1);
        onOpen?.();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [onOpen]);

  useEffect(() => {
    const unlisten = onPopoverWillHide(() => setClosing(true));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return { animKey, closing };
}
