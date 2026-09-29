import { useState } from "react";
import { openSettingsWindow } from "../api";
import { EmptyState } from "../components/EmptyState";
import { IconPencil, IconRefresh, IconSettings } from "../components/Icons";
import { PrList } from "../components/PrList";
import { useConfig } from "../hooks/useConfig";
import { usePopoverAnimation } from "../hooks/usePopoverAnimation";
import { usePullRequests } from "../hooks/usePullRequests";

export function PopoverWindow() {
  const [hideDraft, setHideDraft] = useState(false);
  const { config, isConfigured, reload: reloadConfig } = useConfig();
  // Settings are edited in another window, so re-read them whenever the popover opens.
  const { animKey, closing } = usePopoverAnimation(reloadConfig);
  const { prs, loading, error, lastFetched, refresh } = usePullRequests();

  const draftCount = prs.filter((pr) => pr.draft).length;
  const visiblePrs = prs.filter((pr) => !(hideDraft && pr.draft));

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
          <button
            className="icon-btn"
            title="Settings"
            onClick={() => openSettingsWindow().catch(console.error)}
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
