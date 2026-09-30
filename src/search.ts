import type { PullRequest } from "./types";

/** Every whitespace-separated term must appear in the PR's title, author, repo, branches or number. */
export function matchesQuery(pr: PullRequest, query: string): boolean {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return true;
  const haystack = [
    pr.title,
    pr.author_display_name,
    pr.repo,
    pr.source_branch,
    pr.destination_branch,
    `#${pr.number}`,
  ]
    .join(" ")
    .toLowerCase();
  return terms.every((t) => haystack.includes(t));
}
