import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { PullRequest } from "../types";
import { PrItem } from "./PrItem";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

const pr: PullRequest = {
  provider: "github",
  account_id: "gh",
  repo: "acme/web",
  number: 12,
  title: "Fix login",
  author_display_name: "Maya Chen",
  author_avatar_url: "",
  source_branch: "fix/login",
  destination_branch: "main",
  created_on: "2026-11-20T10:00:00Z",
  updated_on: "2026-11-20T10:00:00Z",
  comment_count: 2,
  html_url: "https://github.com/acme/web/pull/12",
  draft: false,
  approved_by_me: false,
  approvals: 1,
  ci_status: "success",
  merge: null,
};

describe("PrItem", () => {
  it("shows the author on pending review rows", () => {
    render(<PrItem pr={pr} view="review" />);

    expect(screen.getByText("Maya Chen")).toBeInTheDocument();
    expect(screen.queryByText("Approved")).not.toBeInTheDocument();
  });

  it("marks approved rows instead of the author", () => {
    render(<PrItem pr={{ ...pr, approved_by_me: true }} view="review" />);

    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.queryByText("Maya Chen")).not.toBeInTheDocument();
    expect(screen.getByRole("button")).toHaveAccessibleName(/approved by you/);
  });
});
