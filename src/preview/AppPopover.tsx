import { useMemo, useState } from "react";
import { IconRefresh, IconSearch, IconSettings } from "../components/Icons";
import { PrList } from "../components/PrList";
import { matchesTerms, parseQuery } from "../search";
import { myPrs, reviewPrs } from "./data";

/** The real popover markup and components, with injected data and no Tauri calls. */
export function AppPopover() {
  const [tab, setTab] = useState<"review" | "mine">("review");
  const [searching, setSearching] = useState(false);
  const [query, setQuery] = useState("");
  const prs = tab === "review" ? reviewPrs : myPrs;
  const terms = useMemo(() => parseQuery(query), [query]);
  const visible = useMemo(() => prs.filter((pr) => matchesTerms(pr, terms)), [prs, terms]);

  const ready = myPrs.filter((pr) => pr.merge?.state === "ready").length;

  return (
    <div className="popover">
      <header className="popover-header">
        <div className="popover-brand">
          <img className="popover-logo" src="/app-icon.png" alt="" />
          <span className="popover-title">peekr</span>
        </div>
        <div className="popover-header-actions">
          <button
            className={`icon-btn${searching ? " filter-btn--active" : ""}`}
            title="Search ( / )"
            aria-label="Search"
            aria-expanded={searching}
            onClick={() => {
              setSearching((v) => !v);
              setQuery("");
            }}
          >
            <IconSearch size={16} />
          </button>
          <button className="icon-btn" title="Settings" aria-label="Settings">
            <IconSettings size={16} />
          </button>
        </div>
      </header>

      <div className="tabs" role="tablist" aria-label="Pull requests">
        <button
          role="tab"
          aria-selected={tab === "review"}
          className={`tab${tab === "review" ? " tab--active" : ""}`}
          onClick={() => setTab("review")}
        >
          To review <span className="pr-count">{reviewPrs.length}</span>
        </button>
        <button
          role="tab"
          aria-selected={tab === "mine"}
          className={`tab${tab === "mine" ? " tab--active" : ""}`}
          onClick={() => setTab("mine")}
        >
          My PRs
          <span className="pr-count">
            {ready > 0 ? `${ready}/${myPrs.length}` : myPrs.length}
          </span>
        </button>
      </div>

      {searching && (
        <div className="search-bar">
          <IconSearch size={14} className="search-bar-icon" />
          <input
            autoFocus
            aria-label="Search pull requests"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search title, author, repo or branch"
            spellCheck={false}
          />
        </div>
      )}

      <div
        className="popover-content"
        role="tabpanel"
        onClickCapture={(event) => {
          // The real row opens the PR through Tauri; the browser preview stays put.
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <PrList prs={visible} view={tab} />
      </div>

      <footer className="popover-footer">
        <span className="popover-footer-status">Updated 10:58 AM</span>
        <button className="refresh-btn" title="Refresh" aria-label="Refresh">
          <IconRefresh size={14} />
        </button>
      </footer>
    </div>
  );
}
