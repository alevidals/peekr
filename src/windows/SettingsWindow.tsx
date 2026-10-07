import { hideSettingsWindow } from "../api";
import { Settings } from "../components/Settings";

const dismiss = () => hideSettingsWindow().catch(console.error);

export function SettingsWindow() {
  return (
    <div className="settings-window">
      <div className="settings-drag" data-tauri-drag-region />
      <Settings onSaved={dismiss} onClose={dismiss} />
    </div>
  );
}
