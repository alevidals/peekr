import type { ReactNode } from "react";
import { IconSettings, IconCircleCheck, IconSearchOff } from "./Icons";

type StateType = "no-prs" | "no-mine" | "no-matches" | "not-configured";

interface Props {
  type: StateType | "loading";
}

const SKELETON_WIDTHS = [72, 55, 84, 62];

/** Static by nature, so it is built once instead of on every render. */
const loadingSkeleton = (
  <div className="skeleton-list" aria-busy="true" aria-label="Loading">
    {SKELETON_WIDTHS.map((width, i) => (
      <div key={i} className="skeleton-row" style={{ animationDelay: `${i * 90}ms` }}>
        <div className="skeleton skeleton-avatar" />
        <div className="skeleton-lines">
          <div className="skeleton" style={{ width: `${width}%` }} />
          <div className="skeleton skeleton-thin" style={{ width: `${width - 25}%` }} />
          <div className="skeleton skeleton-thin" style={{ width: "34%" }} />
        </div>
      </div>
    ))}
  </div>
);

const STATES: Record<StateType, ReactNode> = {
  "not-configured": (
    <div className="empty-state">
      <div className="empty-state-icon"><IconSettings size={24} /></div>
      <p className="empty-state-title">Connect an account</p>
      <p className="empty-state-text">Add your Bitbucket or GitHub account in settings to start tracking reviews.</p>
    </div>
  ),
  "no-matches": (
    <div className="empty-state">
      <div className="empty-state-icon"><IconSearchOff size={24} /></div>
      <p className="empty-state-title">No matches</p>
      <p className="empty-state-text">No pull requests match your search or filters.</p>
    </div>
  ),
  "no-mine": (
    <div className="empty-state">
      <div className="empty-state-icon"><IconCircleCheck size={24} /></div>
      <p className="empty-state-title">No open PRs</p>
      <p className="empty-state-text">You have no open pull requests in the tracked accounts.</p>
    </div>
  ),
  "no-prs": (
    <div className="empty-state">
      <div className="empty-state-icon empty-state-icon--ok"><IconCircleCheck size={24} /></div>
      <p className="empty-state-title">All caught up</p>
      <p className="empty-state-text">No pull requests are waiting for your review.</p>
    </div>
  ),
};

export function EmptyState({ type }: Props) {
  return type === "loading" ? loadingSkeleton : STATES[type];
}
