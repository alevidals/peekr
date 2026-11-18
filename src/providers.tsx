import type { ReactNode } from "react";
import type { ProviderKind } from "./types";

/** Per-provider UI copy. Adding a provider = one entry here (plus its backend module). */
export interface ProviderMeta {
  label: string;
  /** Not selectable until the backend implements it. */
  enabled: boolean;
  workspaceLabel: string;
  workspaceHint: ReactNode;
  tokenHint: string;
  /** Where the user creates the token; receives the workspace typed so far. */
  tokenUrl: (workspace: string) => string;
  /** Permissions the token needs, shown as a list. */
  tokenScopes: string[];
  reposHint: string;
  reposPlaceholder: string;
  /** When false, leaving the repo list empty watches every repo of the workspace/owner. */
  reposRequired: boolean;
}

export const PROVIDERS: Record<ProviderKind, ProviderMeta> = {
  bitbucket: {
    label: "Bitbucket",
    enabled: true,
    workspaceLabel: "Workspace",
    workspaceHint: (
      <>
        The slug in the URL: bitbucket.org/<strong>workspace</strong>/repo
      </>
    ),
    tokenHint: "Bitbucket Cloud repository or workspace access token",
    tokenUrl: () => "https://id.atlassian.com/manage-profile/security/api-tokens",
    tokenScopes: ["read:pullrequest:bitbucket", "read:user:bitbucket", "read:repository:bitbucket"],
    reposHint: "One slug per line — the name shown in the URL",
    reposPlaceholder: "my-repo\nanother-repo\nthird-repo",
    reposRequired: true,
  },
  github: {
    label: "GitHub",
    enabled: true,
    workspaceLabel: "Organization / user",
    workspaceHint: (
      <>
        github.com/<strong>owner</strong>/repo
      </>
    ),
    tokenHint:
      "Personal access token. If the organization uses SSO, authorize the token for it.",
    tokenUrl: () => "https://github.com/settings/personal-access-tokens/new",
    tokenScopes: [
      "Fine-grained: Pull requests (Read)",
      "Fine-grained: Checks (Read) and Commit statuses (Read) for CI status",
      "Classic: scope repo",
    ],
    reposHint:
      "Optional — leave empty to watch every repository. One repo per line (name or owner/name)",
    reposPlaceholder: "web\napi",
    reposRequired: false,
  },
};

export const PROVIDER_KINDS = Object.keys(PROVIDERS) as ProviderKind[];
