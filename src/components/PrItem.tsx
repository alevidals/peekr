import { openUrl } from "@tauri-apps/plugin-opener";
import type { CiStatus, PullRequest } from "../types";
import { IconBranch, IconCheck, IconMessage, IconX } from "./Icons";

interface Props {
  pr: PullRequest;
  /** Position in the list; drives the staggered entrance. */
  index?: number;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.max(0, Math.floor(diff / 60000));
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

const CI_LABEL: Record<CiStatus, string> = {
  success: "Pipeline passed",
  failed: "Pipeline failed",
  pending: "Pipeline running",
};

function CiBadge({ status }: { status: CiStatus }) {
  return (
    <span className={`ci-badge ci-badge--${status}`} title={CI_LABEL[status]} aria-label={CI_LABEL[status]}>
      {status === "success" && <IconCheck size={10} />}
      {status === "failed" && <IconX size={10} />}
    </span>
  );
}

export function PrItem({ pr, index = 0 }: Props) {
  function handleClick() {
    openUrl(pr.html_url);
  }

  return (
    <button
      className={`pr-item${pr.draft ? " pr-item--draft" : ""}`}
      onClick={handleClick}
      style={{ animationDelay: `${Math.min(index, 8) * 30}ms` }}
    >
      <div className="pr-item-avatar">
        {pr.author_avatar_url ? (
          <img src={pr.author_avatar_url} alt={pr.author_display_name} />
        ) : (
          <span className="pr-item-avatar-fallback">
            {pr.author_display_name.charAt(0).toUpperCase()}
          </span>
        )}
      </div>
      <div className="pr-item-body">
        <div className="pr-item-title">
          {pr.draft && <span className="pr-item-draft-badge">Draft</span>}
          {pr.title}
        </div>
        <div className="pr-item-meta">
          <IconBranch size={12} className="pr-item-branch-icon" />
          <span className="pr-item-branches">
            {pr.source_branch} → {pr.destination_branch}
          </span>
        </div>
        <div className="pr-item-footer">
          <span className="pr-item-author">{pr.author_display_name}</span>
          <div className="pr-item-footer-right">
            {pr.ci_status && <CiBadge status={pr.ci_status} />}
            {pr.approvals > 0 && (
              <span
                className="pr-item-approvals"
                title={`${pr.approvals} ${pr.approvals === 1 ? "approval" : "approvals"}`}
              >
                <IconCheck size={12} /> {pr.approvals}
              </span>
            )}
            <span className="pr-item-time">{timeAgo(pr.created_on)}</span>
            {pr.comment_count > 0 && (
              <span className="pr-item-comments">
                <IconMessage size={13} /> {pr.comment_count}
              </span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}
