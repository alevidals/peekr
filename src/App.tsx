import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { PrList } from "./components/PrList";
import { Settings } from "./components/Settings";
import { EmptyState } from "./components/EmptyState";
import { IconSettings, IconCheck, IconPencil, IconRefresh } from "./components/Icons";
import { usePullRequests } from "./hooks/usePullRequests";
import type { Config } from "./types";
import "./App.css";

const windowLabel = getCurrentWindow().label;

function openSettingsWindow() {
  invoke("open_settings_window").catch(console.error);
}

// ── Settings window ──────────────────────────────────────────────────────────

function SettingsApp() {
  function dismiss() {
    invoke("hide_settings_window").catch(console.error);
  }
  return (
    <div className="settings-window">
      <Settings onSaved={dismiss} onClose={dismiss} />
    </div>
  );
}

// ── Popover window ───────────────────────────────────────────────────────────

function PopoverApp() {
  const [config, setConfig] = useState<Config | null | undefined>(undefined);
  const [hideApproved, setHideApproved] = useState(false);
  const [hideDraft, setHideDraft] = useState(false);
  const [animKey, setAnimKey] = useState(0);
  const [closing, setClosing] = useState(false);
  const { prs, loading, error, lastFetched, refresh } = usePullRequests();

  // Trigger open animation when window gains focus
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) {
          setClosing(false);
          setAnimKey((k) => k + 1);
        }
      })
      .then((fn) => { unlisten = fn; });
    return () => unlisten?.();
  }, []);

  // Play closing animation then hide
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen("popover-will-hide", () => {
        setClosing(true);
      }).then((fn) => { unlisten = fn; });
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    invoke<Config | null>("get_config")
      .then(setConfig)
      .catch(() => setConfig(null));
  }, []);

  const isConfigured = !!config && config.workspace !== "";
  const draftCount = prs.filter((pr) => pr.draft).length;
  const approvedCount = prs.filter((pr) => pr.approved).length;
  const visiblePrs = prs
    .filter((pr) => !(hideApproved && pr.approved))
    .filter((pr) => !(hideDraft && pr.draft));

  function renderContent() {
    if (config === undefined) return <EmptyState type="loading" />;
    if (!isConfigured) return <EmptyState type="not-configured" />;
    if (loading && prs.length === 0) return <EmptyState type="loading" />;
    if (visiblePrs.length === 0) return <EmptyState type="no-prs" />;
    return <PrList prs={visiblePrs} />;
  }

  return (
    <div key={animKey} className={`popover${closing ? " popover--closing" : ""}`}>
      <header className="popover-header">
        <span className="popover-title">peekr</span>
        <div className="popover-header-actions">
          {!loading && prs.length > 0 && (
            <span className="pr-count">{prs.length > 99 ? "99+" : prs.length}</span>
          )}
          {draftCount > 0 && (
            <button
              className={`icon-btn filter-btn${hideDraft ? " filter-btn--active" : ""}`}
              title={hideDraft ? "Show drafts" : "Hide drafts"}
              onClick={() => setHideDraft((v) => !v)}
            >
              <IconPencil size={18} />
            </button>
          )}
          {approvedCount > 0 && (
            <button
              className={`icon-btn filter-btn${hideApproved ? " filter-btn--active" : ""}`}
              title={hideApproved ? "Show approved" : "Hide approved"}
              onClick={() => setHideApproved((v) => !v)}
            >
              <IconCheck size={18} />
            </button>
          )}
          <button
            className="icon-btn"
            title="Settings"
            onClick={openSettingsWindow}
          >
            <IconSettings size={18} />
          </button>
        </div>
      </header>

      {error && <div className="error-banner">⚠ {error}</div>}

      <div className="popover-content">{renderContent()}</div>

      {isConfigured && (
        <footer className="popover-footer">
          <span className="popover-footer-status">
            {loading
              ? "Loading..."
              : lastFetched
              ? lastFetched.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
              : ""}
          </span>
          <button className="refresh-btn" onClick={refresh} title="Refresh">
            <IconRefresh size={16} />
          </button>
        </footer>
      )}
    </div>
  );
}

// ── Root ─────────────────────────────────────────────────────────────────────

export default function App() {
  if (windowLabel === "settings") return <SettingsApp />;
  return <PopoverApp />;
}
