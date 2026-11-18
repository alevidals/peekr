import type { PullRequest } from "./types";

/** A query split into lowercase terms. Normalise once and reuse for every PR. */
export function parseQuery(query: string): string[] {
  return query.toLowerCase().split(/\s+/).filter(Boolean);
}

/** Every term must appear in the PR's title, author, repo, branches or number. */
export function matchesTerms(pr: PullRequest, terms: string[]): boolean {
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
