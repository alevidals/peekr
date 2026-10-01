import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { IconCheck, IconChevron, IconTrash, ProviderIcon } from "./Icons";
import { PROVIDERS, PROVIDER_KINDS } from "../providers";
import type { ProviderKind } from "../types";

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

export const parseRepos = (text: string) =>
  text
    .split("\n")
    .map((r) => r.trim())
    .filter(Boolean);

export const isDraftComplete = (d: AccountDraft) =>
  d.workspace.trim() !== "" &&
  (d.token !== "" || d.hasToken) &&
  (!PROVIDERS[d.kind].reposRequired || parseRepos(d.reposText).length > 0);

interface Props {
  draft: AccountDraft;
  defaultOpen: boolean;
  message: { type: "ok" | "error"; text: string } | null;
  validating: boolean;
  onChange: (patch: Partial<AccountDraft>) => void;
  onValidate: () => void;
  onRemove: () => void;
}

export function AccountForm({
  draft,
  defaultOpen,
  message,
  validating,
  onChange,
  onValidate,
  onRemove,
}: Props) {
  const [open, setOpen] = useState(defaultOpen);
  const meta = PROVIDERS[draft.kind];
  const complete = isDraftComplete(draft);
  const repoCount = parseRepos(draft.reposText).length;
  const summary = `${meta.label} · ${
    repoCount === 0 ? "todos los repos" : `${repoCount} ${repoCount === 1 ? "repo" : "repos"}`
  }`;

  return (
    <section className={`account${open ? " account--open" : ""}`}>
      <div className="account-header">
        <button
          type="button"
          className="account-toggle"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
        >
          <span className={`account-avatar account-avatar--${draft.kind}`}>
            <ProviderIcon kind={draft.kind} size={16} />
          </span>
          <span className="account-heading">
            <span className="account-name">{draft.workspace.trim() || "Nueva cuenta"}</span>
            <span className="account-summary">{summary}</span>
          </span>
          <span
            className={`account-status${complete ? " account-status--ok" : ""}`}
            title={complete ? "Completa" : "Faltan datos"}
          >
            {complete ? <IconCheck size={11} /> : null}
          </span>
          <IconChevron size={14} className="account-chevron" />
        </button>
        <button type="button" className="icon-btn icon-btn--danger" title="Eliminar cuenta" onClick={onRemove}>
          <IconTrash size={15} />
        </button>
      </div>

      {open && (
        <div className="account-body">
          <div className="field">
            <label>Proveedor</label>
            <div className="segmented" role="radiogroup">
              {PROVIDER_KINDS.map((kind) => (
                <button
                  key={kind}
                  type="button"
                  role="radio"
                  aria-checked={draft.kind === kind}
                  className={`segmented-item${draft.kind === kind ? " segmented-item--active" : ""}`}
                  disabled={!PROVIDERS[kind].enabled}
                  onClick={() => draft.kind !== kind && onChange({ kind })}
                >
                  <ProviderIcon kind={kind} size={14} />
                  {PROVIDERS[kind].label}
                </button>
              ))}
            </div>
          </div>

          <div className="field">
            <label>{meta.workspaceLabel}</label>
            <input
              type="text"
              value={draft.workspace}
              onChange={(e) => onChange({ workspace: e.target.value })}
              placeholder="my-workspace"
              spellCheck={false}
            />
            <span className="hint">{meta.workspaceHint}</span>
          </div>

          <div className="field">
            <label>Token</label>
            <input
              type="password"
              value={draft.token}
              onChange={(e) => onChange({ token: e.target.value })}
              placeholder={draft.hasToken ? "Guardado — deja en blanco para no cambiarlo" : "••••••••••••"}
            />
            <span className="hint">{meta.tokenHint}</span>
            <div className="token-help">
              <span className="hint">Permisos necesarios:</span>
              <ul>
                {meta.tokenScopes.map((s) => (
                  <li key={s}>
                    <code>{s}</code>
                  </li>
                ))}
              </ul>
              <button
                type="button"
                className="link-btn"
                onClick={() => openUrl(meta.tokenUrl(draft.workspace))}
              >
                Crear token en {meta.label} ↗
              </button>
            </div>
          </div>

          <div className="field">
            <label>
              Repos a monitorizar
              {!meta.reposRequired && <em>opcional</em>}
            </label>
            <textarea
              value={draft.reposText}
              onChange={(e) => onChange({ reposText: e.target.value })}
              placeholder={meta.reposPlaceholder}
              rows={3}
              spellCheck={false}
            />
            <span className="hint">{meta.reposHint}</span>
          </div>

          <div className="account-actions">
            {message ? (
              <p className={`message message--${message.type}`}>{message.text}</p>
            ) : (
              <span />
            )}
            <button
              type="button"
              className="btn btn--secondary"
              onClick={onValidate}
              disabled={validating || (!draft.token && !draft.hasToken)}
            >
              {validating ? "Comprobando…" : "Validar token"}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
