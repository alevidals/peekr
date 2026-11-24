import { memo } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { CiStatus, MergeState, PullRequest } from "../types";
import { IconBranch, IconCheck, IconMessage, IconX } from "./Icons";

interface Props {
  pr: PullRequest;
  /** Position in the list; drives the staggered entrance. */
  index?: number;
  /** Review rows lead with the author; own rows lead with the merge verdict. */
  view: "review" | "mine";
}

const CI_STATE: Record<CiStatus, string> = {
  success: "passed",
  failed: "failed",
  pending: "running",
};

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.max(0, Math.floor(diff / 60000));
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.floor(hours / 24)}d`;
}

/** GitHub calls them checks (Actions and commit statuses); Bitbucket calls them pipelines. */
function ciLabel(provider: PullRequest["provider"], status: CiStatus): string {
  return `${provider === "github" ? "Checks" : "Pipeline"} ${CI_STATE[status]}`;
}

function CiBadge({ provider, status }: { provider: PullRequest["provider"]; status: CiStatus }) {
  const label = ciLabel(provider, status);
  return (
    <span className={`ci-badge ci-badge--${status}`} title={label} aria-label={label}>
      {status === "success" && <IconCheck size={10} />}
      {status === "failed" && <IconX size={10} />}
    </span>
  );
}

const MERGE_LABEL: Record<MergeState, string> = {
  ready: "Ready",
  waiting: "Waiting",
  blocked: "Blocked",
  conflicts: "Conflicts",
  behind: "Behind",
  draft: "Draft",
  unknown: "Checking",
};

type Tone = "good" | "wait" | "bad" | "muted";

interface Verdict {
  text: string;
  tone: Tone;
  /** Full blockers, for the tooltip: the row only has space for the label and a count. */
  note?: string;
}

function mergeVerdict(pr: PullRequest): Verdict | null {
  const merge = pr.merge;
  // A draft already carries a badge; repeating it as the verdict adds nothing.
  if (!merge || merge.state === "draft") return null;

  const label =
    merge.state === "ready" && pr.provider === "bitbucket" ? "Looks ready" : MERGE_LABEL[merge.state];
  // GitHub keeps a PR mergeable when only non-required checks fail or are still running
  // (`UNSTABLE`); the CI badge shows it, so name the caveat next to the ready state.
  const caveat =
    merge.state === "ready" && pr.ci_status === "failed"
      ? "Other checks failing"
      : merge.state === "ready" && pr.ci_status === "pending"
        ? "Other checks running"
        : null;
  const note = [...merge.blockers, ...(caveat ? [caveat] : [])].join(", ");
  const tone: Tone =
    merge.state === "ready"
      ? "good"
      : merge.state === "waiting" || merge.state === "unknown"
        ? "wait"
        : "bad";

  return {
    text: merge.blockers.length > 1 ? `${label} ${merge.blockers.length}` : label,
    tone,
    note: note || undefined,
  };
}

export const PrItem = memo(function PrItem({ pr, index = 0, view }: Props) {
  function handleClick() {
    openUrl(pr.html_url);
  }

  const verdict = view === "mine" ? mergeVerdict(pr) : null;
  const updated = timeAgo(pr.updated_on);
  const label = [
    `${pr.title}, #${pr.number}`,
    pr.draft ? "draft" : null,
    `by ${pr.author_display_name}`,
    `${pr.source_branch} to ${pr.destination_branch}`,
    pr.approved_by_me ? "approved by you" : null,
    verdict ? (verdict.note ? `${verdict.text}: ${verdict.note}` : verdict.text) : null,
    pr.ci_status ? ciLabel(pr.provider, pr.ci_status) : null,
    pr.approvals > 0
      ? `${pr.approvals} ${pr.approvals === 1 ? "approval" : "approvals"}`
      : null,
    pr.comment_count > 0 ? `${pr.comment_count} comments` : null,
    `updated ${updated} ago`,
  ]
    .filter(Boolean)
    .join(", ");

  return (
    <button
      className="pr-item"
      onClick={handleClick}
      style={{ animationDelay: `${Math.min(index, 8) * 30}ms` }}
      aria-label={label}
    >
      <span className="pr-item-avatar" aria-hidden="true">
        {pr.author_avatar_url ? (
          <img
            src={pr.author_avatar_url}
            alt=""
            width={28}
            height={28}
            loading="lazy"
            decoding="async"
          />
        ) : (
          <span className="pr-item-avatar-fallback">
            {pr.author_display_name.charAt(0).toUpperCase()}
          </span>
        )}
      </span>

      <span className="pr-item-body" aria-hidden="true">
        <span className="pr-item-top">
          <span className="pr-item-heading">
            {pr.draft && <span className="pr-item-draft-badge">Draft</span>}
            <span className="pr-item-title" title={pr.title}>
              {pr.title}
            </span>
            <span className="pr-item-number">#{pr.number}</span>
          </span>

          {view === "mine" ? (
            verdict && (
              <span
                className={`pr-item-status pr-item-status--${verdict.tone}`}
                title={verdict.note}
              >
                {verdict.text}
              </span>
            )
          ) : pr.approved_by_me ? (
            <span
              className="pr-item-status pr-item-status--good"
              title={`Approved by you · ${pr.author_display_name}`}
            >
              Approved
            </span>
          ) : (
            <span className="pr-item-author" title={pr.author_display_name}>
              {pr.author_display_name}
            </span>
          )}
        </span>

        <span className="pr-item-sub">
          <span className="pr-item-meta">
            <IconBranch size={11} className="pr-item-branch-icon" />
            <span
              className="pr-item-branches"
              title={`${pr.source_branch} → ${pr.destination_branch}`}
            >
              {pr.source_branch} to {pr.destination_branch}
            </span>
          </span>

          <span className="pr-item-right">
            {pr.ci_status && <CiBadge provider={pr.provider} status={pr.ci_status} />}
            {pr.approvals > 0 && (
              <span
                className="pr-item-approvals"
                title={`${pr.approvals} ${pr.approvals === 1 ? "approval" : "approvals"}`}
              >
                <IconCheck size={11} /> {pr.approvals}
              </span>
            )}
            {pr.comment_count > 0 && (
              <span className="pr-item-comments">
                <IconMessage size={11} /> {pr.comment_count}
              </span>
            )}
            <span className="pr-item-time">{updated}</span>
          </span>
        </span>
      </span>
    </button>
  );
});
