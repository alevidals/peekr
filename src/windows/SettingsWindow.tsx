import { hideSettingsWindow } from "../api";
import { Settings } from "../components/Settings";

const dismiss = () => hideSettingsWindow().catch(console.error);

export function SettingsWindow() {
  return (
    <div className="settings-window">
      <Settings onSaved={dismiss} onClose={dismiss} />
    </div>
  );
}
