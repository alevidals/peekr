import type { ReactNode } from "react";
import { IconSettings, IconCircleCheck, IconSearchOff } from "./Icons";

type StateType = "no-prs" | "no-mine" | "no-matches" | "not-configured";

interface Props {
  type: StateType | "loading";
  /** Optional call to action rendered under the copy, inside the centred column. */
  action?: ReactNode;
}

interface StateContent {
  icon: ReactNode;
  title: string;
  text: string;
  ok?: boolean;
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

const STATES: Record<StateType, StateContent> = {
  "not-configured": {
    icon: <IconSettings size={24} />,
    title: "Connect an account",
    text: "Add your Bitbucket or GitHub account in settings to start tracking reviews.",
  },
  "no-matches": {
    icon: <IconSearchOff size={24} />,
    title: "No matches",
    text: "No pull requests match your search or filters.",
  },
  "no-mine": {
    icon: <IconCircleCheck size={24} />,
    title: "No open PRs",
    text: "You have no open pull requests in the tracked accounts.",
  },
  "no-prs": {
    icon: <IconCircleCheck size={24} />,
    title: "All caught up",
    text: "No pull requests are waiting for your review.",
    ok: true,
  },
};

export function EmptyState({ type, action }: Props) {
  if (type === "loading") return loadingSkeleton;
  const state = STATES[type];
  return (
    <div className="empty-state">
      <div className={`empty-state-icon${state.ok ? " empty-state-icon--ok" : ""}`}>{state.icon}</div>
      <p className="empty-state-title">{state.title}</p>
      <p className="empty-state-text">{state.text}</p>
      {action}
    </div>
  );
}
