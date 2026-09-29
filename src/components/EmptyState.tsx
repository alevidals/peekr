import { IconSettings, IconCircleCheck } from "./Icons";

interface Props {
  type: "no-prs" | "not-configured" | "loading";
}

export function EmptyState({ type }: Props) {
  if (type === "loading") {
    return (
      <div className="empty-state">
        <div className="spinner" />
        <p>Loading…</p>
      </div>
    );
  }

  if (type === "not-configured") {
    return (
      <div className="empty-state">
        <IconSettings size={28} className="empty-state-icon" />
        <p>Configure your Bitbucket account to get started.</p>
      </div>
    );
  }

  return (
    <div className="empty-state">
      <IconCircleCheck size={28} className="empty-state-icon" />
      <p>No PRs to review</p>
    </div>
  );
}
