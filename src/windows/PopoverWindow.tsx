import { useCallback, useDeferredValue, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { hidePopover, openSettingsWindow } from "../api";
import { EmptyState } from "../components/EmptyState";
import { IconAlert, IconCircleCheck, IconPencil, IconRefresh, IconSearch, IconSettings, IconX } from "../components/Icons";
import { PrList } from "../components/PrList";
import { matchesTerms, parseQuery } from "../search";
import { useConfig } from "../hooks/useConfig";
import { usePopoverAnimation } from "../hooks/usePopoverAnimation";
import { usePullRequests } from "../hooks/usePullRequests";

/** Restart the CSS entrance animation on the same node, without remounting the list. */
function restartAnimation(el: HTMLElement) {
  el.style.animation = "none";
  void el.offsetWidth;
  el.style.animation = "";
}

export function PopoverWindow() {
  const [tab, setTab] = useState<"review" | "mine">("review");
  const [hideDraft, setHideDraft] = useState(false);
  const [onlyPassing, setOnlyPassing] = useState(false);
  const [searching, setSearching] = useState(false);
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);

  const closeSearch = useCallback(() => {
    setSearching(false);
    setQuery("");
  }, []);

  const { config, isConfigured, reload: reloadConfig } = useConfig();
  // Settings are edited in another window, so re-read them whenever the popover opens.
  // Each opening also starts with a clean search.
  const onOpen = useCallback(() => {
    reloadConfig();
    closeSearch();
  }, [reloadConfig, closeSearch]);
  const { animKey, closing } = usePopoverAnimation(onOpen);
  const { prs: reviewPrs, mine, loading, error, lastFetched, refresh } = usePullRequests();
  const prs = tab === "review" ? reviewPrs : mine;

  const hasPrs = prs.length > 0;
  const activeQuery = searching ? query : "";
  // Filtering runs on the deferred value so typing stays responsive on long lists. When the
  // search is closed the terms are empty immediately, instead of keeping the last query for
  // a frame while the deferred value catches up.
  const deferredQuery = useDeferredValue(activeQuery);
  const terms = useMemo(
    () => (activeQuery === "" ? [] : parseQuery(deferredQuery)),
    [activeQuery, deferredQuery]
  );
  const visiblePrs = useMemo(
    () =>
      prs.filter(
        (pr) =>
          !(hideDraft && pr.draft) &&
          !(onlyPassing && pr.ci_status !== "success") &&
          matchesTerms(pr, terms)
      ),
    [prs, hideDraft, onlyPassing, terms]
  );

  // The header badges need three counts from the same lists: one pass instead of three.
  const { draftCount, hasCi, readyCount } = useMemo(() => {
    let drafts = 0;
    let ci = false;
    for (const pr of prs) {
      if (pr.draft) drafts++;
      if (pr.ci_status !== null) ci = true;
    }
    let ready = 0;
    for (const pr of mine) if (pr.merge?.state === "ready") ready++;
    return { draftCount: drafts, hasCi: ci, readyCount: ready };
  }, [prs, mine]);

  // Replay the entrance animation without remounting the whole tree, and start at the top.
  // The webview mounts hidden, so the first focus also needs a restart to be visible. This
  // runs before paint to avoid a frame at full size followed by the animation's first frame.
  useLayoutEffect(() => {
    if (closing) return;
    if (popoverRef.current) restartAnimation(popoverRef.current);
    if (contentRef.current) contentRef.current.scrollTop = 0;
  }, [animKey, closing]);

  // Escape hides the popover through the same animated path as click-outside; "/" opens
  // the search from anywhere. Depending on the boolean (not the list length) keeps the
  // listener from resubscribing every time the PR list changes.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        // While the search is open, its input closes the search first.
        if (e.target === searchRef.current) return;
        e.preventDefault();
        hidePopover().catch(console.error);
        return;
      }
      if (e.key !== "/" || !hasPrs) return;
      const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
      if (typing) return;
      e.preventDefault();
      setSearching(true);
      searchRef.current?.focus();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [hasPrs]);

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
    <div ref={popoverRef} className={`popover${closing ? " popover--closing" : ""}`}>
      <header className="popover-header">
        <div className="popover-brand">
          <img className="popover-logo" src="/app-icon.png" alt="" />
          <span className="popover-title">peekr</span>
        </div>
        <div className="popover-header-actions">
          {hasPrs && (
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

      <div ref={contentRef} className="popover-content">{renderContent()}</div>

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
            <span className={loading ? "spin" : undefined}>
              <IconRefresh size={14} />
            </span>
          </button>
        </footer>
      )}
    </div>
  );
}
