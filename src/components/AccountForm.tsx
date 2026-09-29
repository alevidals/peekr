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
  message: { type: "ok" | "error"; text: string } | null;
  validating: boolean;
  onChange: (patch: Partial<AccountDraft>) => void;
  onValidate: () => void;
  onRemove: () => void;
}

export function AccountForm({ draft, message, validating, onChange, onValidate, onRemove }: Props) {
  const meta = PROVIDERS[draft.kind];

  return (
    <div className="settings-account">
      <div className="settings-account-header">
        <div className="settings-field">
          <label>Proveedor</label>
          <select
            value={draft.kind}
            onChange={(e) => onChange({ kind: e.target.value as ProviderKind })}
          >
            {PROVIDER_KINDS.map((kind) => (
              <option key={kind} value={kind} disabled={!PROVIDERS[kind].enabled}>
                {PROVIDERS[kind].label}
                {PROVIDERS[kind].enabled ? "" : " (próximamente)"}
              </option>
            ))}
          </select>
        </div>
        <button className="btn-danger" onClick={onRemove}>
          Eliminar
        </button>
      </div>

      <div className="settings-field">
        <label>{meta.workspaceLabel}</label>
        <input
          type="text"
          value={draft.workspace}
          onChange={(e) => onChange({ workspace: e.target.value })}
          placeholder="my-workspace"
        />
        <span className="settings-hint">{meta.workspaceHint}</span>
      </div>

      <div className="settings-field">
        <label>Token</label>
        <input
          type="password"
          value={draft.token}
          onChange={(e) => onChange({ token: e.target.value })}
          placeholder={draft.hasToken ? "dejar en blanco para no cambiar" : "••••••••••••"}
        />
        <span className="settings-hint">{meta.tokenHint}</span>
      </div>

      <div className="settings-field">
        <label>Repos a monitorizar{meta.reposRequired ? "" : " (opcional)"}</label>
        <textarea
          value={draft.reposText}
          onChange={(e) => onChange({ reposText: e.target.value })}
          placeholder={meta.reposPlaceholder}
          rows={4}
          spellCheck={false}
        />
        <span className="settings-hint">{meta.reposHint}</span>
      </div>

      {message && (
        <p className={`settings-message settings-message--${message.type}`}>{message.text}</p>
      )}

      <div className="settings-actions">
        <button
          className="btn-secondary"
          onClick={onValidate}
          disabled={validating || (!draft.token && !draft.hasToken)}
        >
          {validating ? "Comprobando…" : "Validar token"}
        </button>
      </div>
    </div>
  );
}
