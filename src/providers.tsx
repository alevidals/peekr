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
        El slug de la URL: bitbucket.org/<strong>workspace</strong>/repo
      </>
    ),
    tokenHint: "Repository o Workspace Access Token de Bitbucket Cloud",
    tokenUrl: () => "https://id.atlassian.com/manage-profile/security/api-tokens",
    tokenScopes: ["read:pullrequest:bitbucket", "read:user:bitbucket", "read:repository:bitbucket"],
    reposHint: "Un slug por línea — el nombre que aparece en la URL",
    reposPlaceholder: "my-repo\nanother-repo\nthird-repo",
    reposRequired: true,
  },
  github: {
    label: "GitHub",
    enabled: true,
    workspaceLabel: "Organización / usuario",
    workspaceHint: (
      <>
        github.com/<strong>owner</strong>/repo
      </>
    ),
    tokenHint:
      "Personal Access Token. Si la organización usa SSO, autoriza el token para ella.",
    tokenUrl: () => "https://github.com/settings/personal-access-tokens/new",
    tokenScopes: [
      "Fine-grained: Pull requests (Read)",
      "Fine-grained: Checks (Read) y Commit statuses (Read) para el estado del CI",
      "Classic: scope repo",
    ],
    reposHint:
      "Opcional — déjalo vacío para vigilar todos los repos. Un repo por línea (nombre u owner/nombre)",
    reposPlaceholder: "web\napi",
    reposRequired: false,
  },
};

export const PROVIDER_KINDS = Object.keys(PROVIDERS) as ProviderKind[];
