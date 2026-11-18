export type ProviderKind = "bitbucket" | "github";

export type Theme = "system" | "light" | "dark";

export type CiStatus = "success" | "failed" | "pending";

export type MergeState = "ready" | "waiting" | "blocked" | "conflicts" | "behind" | "draft" | "unknown";

export interface MergeStatus {
  state: MergeState;
  /** Why the PR cannot be merged yet; empty when ready. */
  blockers: string[];
}

export interface PullRequest {
  provider: ProviderKind;
  account_id: string;
  /** Repo name within the account; PR numbers are only unique per repo. */
  repo: string;
  number: number;
  title: string;
  author_display_name: string;
  author_avatar_url: string;
  source_branch: string;
  destination_branch: string;
  created_on: string;
  updated_on: string;
  comment_count: number;
  html_url: string;
  draft: boolean;
  approvals: number;
  /** Absent when the PR has no checks or they could not be read. */
  ci_status: CiStatus | null;
  /** Only present on the user's own PRs. */
  merge: MergeStatus | null;
}

export type NamedPalette =
  | "oneDark"
  | "gruvboxDark"
  | "gruvboxLight"
  | "nord"
  | "dracula"
  | "solarizedLight"
  | "tokyoNight"
  | "catppuccinMocha";

export type Palette = "indigo" | "graphite" | "green" | "amber" | "contrast" | NamedPalette;
export type Density = "comfortable" | "compact";
export type TextSize = "small" | "medium" | "large";

export interface Appearance {
  palette: Palette;
  /** `#rrggbb` overriding the palette's accent. */
  accent: string | null;
  density: Density;
  text_size: TextSize;
}

export const DEFAULT_APPEARANCE: Appearance = {
  palette: "indigo",
  accent: null,
  density: "comfortable",
  text_size: "medium",
};

export interface AccountConfig {
  id: string;
  kind: ProviderKind;
  workspace: string;
  repos: string[];
  has_token: boolean;
}

export interface Config {
  polling_interval_secs: number;
  theme: Theme;
  appearance: Appearance;
  accounts: AccountConfig[];
}

/** What the settings window sends back for each account. */
export interface AccountInput {
  id?: string;
  kind: ProviderKind;
  workspace: string;
  repos: string[];
  /** Empty keeps the stored token. */
  token?: string;
}

export const prKey = (pr: PullRequest) => `${pr.account_id}/${pr.repo}#${pr.number}`;
