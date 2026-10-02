import { useCallback, useEffect, useRef, useState } from "react";
import { openSettingsWindow } from "../api";
import { EmptyState } from "../components/EmptyState";
import { IconAlert, IconCircleCheck, IconPencil, IconRefresh, IconSearch, IconSettings, IconX } from "../components/Icons";
import { PrList } from "../components/PrList";
import { matchesQuery } from "../search";
import { useConfig } from "../hooks/useConfig";
import { usePopoverAnimation } from "../hooks/usePopoverAnimation";
import { usePullRequests } from "../hooks/usePullRequests";

export function PopoverWindow() {
  const [tab, setTab] = useState<"review" | "mine">("review");
  const [hideDraft, setHideDraft] = useState(false);
  const [onlyPassing, setOnlyPassing] = useState(false);
  const [searching, setSearching] = useState(false);
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  function closeSearch() {
    setSearching(false);
    setQuery("");
  }

  const { config, isConfigured, reload: reloadConfig } = useConfig();
  // Settings are edited in another window, so re-read them whenever the popover opens.
  // Each opening also starts with a clean search.
  const onOpen = useCallback(() => {
    reloadConfig();
    closeSearch();
  }, [reloadConfig]);
  const { animKey, closing } = usePopoverAnimation(onOpen);
  const { prs: reviewPrs, mine, loading, error, lastFetched, refresh } = usePullRequests();
  const prs = tab === "review" ? reviewPrs : mine;
  const readyCount = mine.filter((pr) => pr.merge?.state === "ready").length;

  const draftCount = prs.filter((pr) => pr.draft).length;
  const hasCi = prs.some((pr) => pr.ci_status !== null);
  const activeQuery = searching ? query : "";
  const visiblePrs = prs.filter(
    (pr) =>
      !(hideDraft && pr.draft) &&
      !(onlyPassing && pr.ci_status !== "success") &&
      matchesQuery(pr, activeQuery)
  );

  // "/" opens the search from anywhere in the popover.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
      if (e.key === "/" && !typing && prs.length > 0) {
        e.preventDefault();
        setSearching(true);
        searchRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [prs.length]);

  function renderContent() {
    if (config === undefined) return <EmptyState type="loading" />;
    if (!isConfigured) return <EmptyState type="not-configured" />;
    if (loading && prs.length === 0) return <EmptyState type="loading" />;
    if (visiblePrs.length === 0) {
      return <EmptyState type={prs.length > 0 ? "no-matches" : tab === "review" ? "no-prs" : "no-mine"} />;
    }
    return <PrList prs={visiblePrs} />;
  }

  return (
    <div key={animKey} className={`popover${closing ? " popover--closing" : ""}`}>
      <header className="popover-header">
        <div className="popover-brand">
          <span className="popover-logo" />
          <span className="popover-title">peekr</span>
        </div>
        <div className="popover-header-actions">
          {prs.length > 0 && (
            <button
              className={`icon-btn${searching ? " filter-btn--active" : ""}`}
              title="Search ( / )"
              onClick={() => (searching ? closeSearch() : setSearching(true))}
            >
              <IconSearch size={16} />
            </button>
          )}
          {hasCi && (
            <button
              className={`icon-btn${onlyPassing ? " filter-btn--active" : ""}`}
              title={onlyPassing ? "Show all pipelines" : "Only PRs with passing pipeline"}
              aria-pressed={onlyPassing}
              onClick={() => setOnlyPassing((v) => !v)}
            >
              <IconCircleCheck size={16} />
            </button>
          )}
          {draftCount > 0 && (
            <button
              className={`icon-btn filter-btn${hideDraft ? " filter-btn--active" : ""}`}
              title={hideDraft ? "Show drafts" : "Hide drafts"}
              onClick={() => setHideDraft((v) => !v)}
            >
              <IconPencil size={16} />
            </button>
          )}
          <button
            className="icon-btn"
            title="Settings"
            onClick={() => openSettingsWindow().catch(console.error)}
          >
            <IconSettings size={16} />
          </button>
        </div>
      </header>

      {isConfigured && (
        <nav className="tabs" role="tablist">
          <button
            role="tab"
            aria-selected={tab === "review"}
            className={`tab${tab === "review" ? " tab--active" : ""}`}
            onClick={() => setTab("review")}
          >
            To review
            {reviewPrs.length > 0 && <span className="pr-count">{reviewPrs.length > 99 ? "99+" : reviewPrs.length}</span>}
          </button>
          <button
            role="tab"
            aria-selected={tab === "mine"}
            className={`tab${tab === "mine" ? " tab--active" : ""}`}
            onClick={() => setTab("mine")}
          >
            My PRs
            {mine.length > 0 && (
              <span className={`pr-count${readyCount > 0 ? " pr-count--ready" : ""}`} title={readyCount > 0 ? `${readyCount} ready to merge` : undefined}>
                {readyCount > 0 ? `${readyCount}/${mine.length}` : mine.length}
              </span>
            )}
          </button>
        </nav>
      )}

      {searching && (
        <div className="search-bar">
          <IconSearch size={14} className="search-bar-icon" />
          <input
            ref={searchRef}
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.stopPropagation();
                closeSearch();
              }
            }}
            placeholder="Search title, author, repo or branch"
            spellCheck={false}
          />
          {query && (
            <button className="icon-btn search-bar-clear" title="Clear" onClick={() => setQuery("")}>
              <IconX size={12} />
            </button>
          )}
        </div>
      )}

      {error && <div className="error-banner"><IconAlert size={13} /> <span>{error}</span></div>}

      <div className="popover-content">{renderContent()}</div>

      {isConfigured && (
        <footer className="popover-footer">
          <span className="popover-footer-status">
            {loading
              ? "Syncing…"
              : lastFetched
              ? `Updated ${lastFetched.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`
              : ""}
          </span>
          <button className="refresh-btn" onClick={refresh} title="Refresh">
            <IconRefresh size={14} className={loading ? "spin" : undefined} />
          </button>
        </footer>
      )}
    </div>
  );
}
