import { memo } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { CiStatus, MergeState, PullRequest } from "../types";
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

const MERGE_LABEL: Record<MergeState, string> = {
  ready: "Ready to merge",
  waiting: "Waiting",
  blocked: "Blocked",
  conflicts: "Conflicts",
  behind: "Behind base",
  draft: "Draft",
  unknown: "Checking…",
};

/** Bitbucket has no merge verdict, so its "ready" is an inference from checks and approvals. */
function MergePill({ pr }: { pr: PullRequest }) {
  const merge = pr.merge;
  if (!merge || merge.state === "draft") return null;
  const label = merge.state === "ready" && pr.provider === "bitbucket" ? "Looks ready" : MERGE_LABEL[merge.state];
  // Do not repeat the label as its own blocker ("Behind base · Behind base").
  const blockers = merge.blockers.filter((blocker) => blocker.toLowerCase() !== label.toLowerCase());
  return (
    <span className={`merge-pill merge-pill--${merge.state}`} title={merge.blockers.join("\n") || label}>
      <span className="merge-pill-label">{label}</span>
      {blockers.length > 0 && <span className="merge-pill-blockers">{blockers.join(", ")}</span>}
    </span>
  );
}

export const PrItem = memo(function PrItem({ pr, index = 0 }: Props) {
  function handleClick() {
    openUrl(pr.html_url);
  }

  return (
    <button
      className={`pr-item${pr.draft ? " pr-item--draft" : ""}`}
      onClick={handleClick}
      style={{ animationDelay: `${Math.min(index, 8) * 30}ms` }}
    >
      <span className="pr-item-avatar">
        {pr.author_avatar_url ? (
          <img
            src={pr.author_avatar_url}
            alt=""
            width={30}
            height={30}
            loading="lazy"
            decoding="async"
          />
        ) : (
          <span className="pr-item-avatar-fallback">
            {pr.author_display_name.charAt(0).toUpperCase()}
          </span>
        )}
      </span>
      <span className="pr-item-body">
        <span className="pr-item-title">
          {pr.draft && <span className="pr-item-draft-badge">Draft</span>}
          {pr.title}
        </span>
        <span className="pr-item-meta">
          <IconBranch size={12} className="pr-item-branch-icon" />
          <span className="pr-item-branches">
            {pr.source_branch} → {pr.destination_branch}
          </span>
        </span>
        <MergePill pr={pr} />
        <span className="pr-item-footer">
          <span className="pr-item-author">{pr.author_display_name}</span>
          <span className="pr-item-footer-right">
            {pr.ci_status && <CiBadge status={pr.ci_status} />}
            {pr.approvals > 0 && (
              <span
                className="pr-item-approvals"
                title={`${pr.approvals} ${pr.approvals === 1 ? "approval" : "approvals"}`}
              >
                <IconCheck size={12} /> {pr.approvals}
              </span>
            )}
            <span className="pr-item-time">{timeAgo(pr.updated_on)}</span>
            {pr.comment_count > 0 && (
              <span className="pr-item-comments">
                <IconMessage size={13} /> {pr.comment_count}
              </span>
            )}
          </span>
        </span>
      </span>
    </button>
  );
});
