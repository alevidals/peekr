import { getCurrentWindow } from "@tauri-apps/api/window";
import { PopoverWindow } from "./windows/PopoverWindow";
import { SettingsWindow } from "./windows/SettingsWindow";
import "./App.css";

// Both windows load the same bundle; the window label decides what to render.
const windowLabel = getCurrentWindow().label;

export default function App() {
  return windowLabel === "settings" ? <SettingsWindow /> : <PopoverWindow />;
}
