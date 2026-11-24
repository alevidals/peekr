import { describe, expect, it } from "vitest";
import { matchesTerms, parseQuery } from "./search";
import type { PullRequest } from "./types";

const pr: PullRequest = {
  provider: "github",
  account_id: "account",
  repo: "acme/web",
  number: 42,
  title: "Fix login flow",
  author_display_name: "Alejandro",
  author_avatar_url: "",
  source_branch: "fix/login",
  destination_branch: "main",
  created_on: "",
  updated_on: "",
  comment_count: 0,
  html_url: "https://example.test",
  draft: false,
  approved_by_me: false,
  approvals: 0,
  ci_status: null,
  merge: null,
};

describe("search", () => {
  it("normalises and splits the query", () => {
    expect(parseQuery("  Fix   LOGIN ")).toEqual(["fix", "login"]);
    expect(parseQuery("")).toEqual([]);
  });

  it("requires every term to match somewhere in the PR", () => {
    expect(matchesTerms(pr, parseQuery("login alejandro"))).toBe(true);
    expect(matchesTerms(pr, parseQuery("login #42"))).toBe(true);
    expect(matchesTerms(pr, parseQuery("login missing"))).toBe(false);
    expect(matchesTerms(pr, [])).toBe(true);
  });

  it("matches branches and repository names case-insensitively", () => {
    expect(matchesTerms(pr, parseQuery("fix/login"))).toBe(true);
    expect(matchesTerms(pr, parseQuery("ACME/WEB"))).toBe(true);
  });
});
