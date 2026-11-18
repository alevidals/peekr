import { lazy, Suspense } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

// Both windows load the same entry; only the one being opened is downloaded.
const PopoverWindow = lazy(() =>
  import("./windows/PopoverWindow").then((m) => ({ default: m.PopoverWindow }))
);
const SettingsWindow = lazy(() =>
  import("./windows/SettingsWindow").then((m) => ({ default: m.SettingsWindow }))
);

// The window label decides what to render.
const windowLabel = getCurrentWindow().label;

export default function App() {
  return (
    <Suspense fallback={null}>
      {windowLabel === "settings" ? <SettingsWindow /> : <PopoverWindow />}
    </Suspense>
  );
}
