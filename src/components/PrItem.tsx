import { openUrl } from "@tauri-apps/plugin-opener";
import type { PullRequest } from "../types";
import { IconMessage } from "./Icons";

interface Props {
  pr: PullRequest;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function PrItem({ pr }: Props) {
  function handleClick() {
    openUrl(pr.html_url);
  }

  return (
    <button
      className={`pr-item${pr.approved ? " pr-item--approved" : ""}${pr.draft ? " pr-item--draft" : ""}`}
      onClick={handleClick}
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
          <span className="pr-item-repo">{pr.repo_slug}</span>
          <span className="pr-item-branches">
            {pr.source_branch} → {pr.destination_branch}
          </span>
        </div>
        <div className="pr-item-footer">
          <span className="pr-item-author">{pr.author_display_name}</span>
          <div className="pr-item-footer-right">
            <span className="pr-item-time">{timeAgo(pr.created_on)}</span>
            {pr.comment_count > 0 && (
              <span className="pr-item-comments">
                <IconMessage size={13} /> {pr.comment_count}
              </span>
            )}
            {pr.approved && (
              <span className="pr-item-approved" title="Aprobada">✓</span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}
