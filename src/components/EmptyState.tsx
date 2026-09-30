import { IconSettings, IconCircleCheck, IconSearchOff } from "./Icons";

interface Props {
  type: "no-prs" | "no-matches" | "not-configured" | "loading";
}

export function EmptyState({ type }: Props) {
  if (type === "loading") {
    return (
      <div className="skeleton-list" aria-busy="true" aria-label="Loading">
        {[72, 55, 84, 62].map((width, i) => (
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
  }

  if (type === "not-configured") {
    return (
      <div className="empty-state">
        <div className="empty-state-icon"><IconSettings size={24} /></div>
        <p className="empty-state-title">Connect an account</p>
        <p className="empty-state-text">Add your Bitbucket or GitHub account in settings to start tracking reviews.</p>
      </div>
    );
  }

  if (type === "no-matches") {
    return (
      <div className="empty-state">
        <div className="empty-state-icon"><IconSearchOff size={24} /></div>
        <p className="empty-state-title">No matches</p>
        <p className="empty-state-text">No pull requests match your search or filters.</p>
      </div>
    );
  }

  return (
    <div className="empty-state">
      <div className="empty-state-icon empty-state-icon--ok"><IconCircleCheck size={24} /></div>
      <p className="empty-state-title">All caught up</p>
      <p className="empty-state-text">No pull requests are waiting for your review.</p>
    </div>
  );
}
