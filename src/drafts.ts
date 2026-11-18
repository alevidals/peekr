import { PROVIDERS } from "./providers";
import type { AccountInput, Config, ProviderKind } from "./types";

/** An account being edited; `token` is only what the user typed this session. */
export interface AccountDraft {
  /** Local React key; stable across edits, unlike `id`, which new accounts don't have yet. */
  key: string;
  id?: string;
  kind: ProviderKind;
  workspace: string;
  token: string;
  hasToken: boolean;
  reposText: string;
}

let draftCounter = 0;

export function blankDraft(kind: ProviderKind = "bitbucket"): AccountDraft {
  return {
    key: `new-${draftCounter++}`,
    kind,
    workspace: "",
    token: "",
    hasToken: false,
    reposText: "",
  };
}

export const parseRepos = (text: string) =>
  text
    .split("\n")
    .map((r) => r.trim())
    .filter(Boolean);

export const isDraftComplete = (d: AccountDraft) =>
  d.workspace.trim() !== "" &&
  (d.token !== "" || d.hasToken) &&
  (!PROVIDERS[d.kind].reposRequired || parseRepos(d.reposText).length > 0);

/** The settings window's editing state from what is stored on disk. */
export function configToDrafts(config: Config): AccountDraft[] {
  if (config.accounts.length === 0) return [blankDraft()];
  return config.accounts.map((a) => ({
    key: a.id,
    id: a.id,
    kind: a.kind,
    workspace: a.workspace,
    token: "",
    hasToken: a.has_token,
    reposText: a.repos.join("\n"),
  }));
}

export function draftsToAccounts(drafts: AccountDraft[]): AccountInput[] {
  return drafts.map((d) => ({
    id: d.id,
    kind: d.kind,
    workspace: d.workspace.trim(),
    repos: parseRepos(d.reposText),
    token: d.token || undefined,
  }));
}
