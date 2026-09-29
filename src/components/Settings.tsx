import { useEffect, useState } from "react";
import { getConfig, saveConfig, validateCredentials } from "../api";
import type { AccountInput } from "../types";
import { AccountForm, isDraftComplete, parseRepos, type AccountDraft } from "./AccountForm";

interface Props {
  onSaved: () => void;
  onClose?: () => void;
}

type Message = { type: "ok" | "error"; text: string };

let draftCounter = 0;
const blankDraft = (): AccountDraft => ({
  key: `new-${draftCounter++}`,
  kind: "bitbucket",
  workspace: "",
  token: "",
  hasToken: false,
  reposText: "",
});

export function Settings({ onSaved, onClose }: Props) {
  const [drafts, setDrafts] = useState<AccountDraft[]>([]);
  const [pollingInterval, setPollingInterval] = useState(120);
  const [saving, setSaving] = useState(false);
  const [validatingKey, setValidatingKey] = useState<string | null>(null);
  // Per-account messages, keyed by draft key; "" is the form-level message.
  const [messages, setMessages] = useState<Record<string, Message | null>>({});

  useEffect(() => {
    getConfig().then((config) => {
      setPollingInterval(config.polling_interval_secs);
      setDrafts(
        config.accounts.length === 0
          ? [blankDraft()]
          : config.accounts.map((a) => ({
              key: a.id,
              id: a.id,
              kind: a.kind,
              workspace: a.workspace,
              token: "",
              hasToken: a.has_token,
              reposText: a.repos.join("\n"),
            }))
      );
    });
  }, []);

  const setMessage = (key: string, message: Message | null) =>
    setMessages((m) => ({ ...m, [key]: message }));

  const updateDraft = (key: string, patch: Partial<AccountDraft>) => {
    setDrafts((ds) =>
      ds.map((d) => {
        if (d.key !== key) return d;
        // Workspace, token and repos mean nothing for another provider: start a fresh
        // account (no id, so the old one and its stored token are dropped on save).
        if (patch.kind && patch.kind !== d.kind) {
          return { ...blankDraft(), key: d.key, kind: patch.kind };
        }
        return { ...d, ...patch };
      })
    );
    if (patch.kind) setMessage(key, null);
  };

  async function handleValidate(draft: AccountDraft) {
    setValidatingKey(draft.key);
    setMessage(draft.key, null);
    try {
      const userId = await validateCredentials(draft.kind, draft.id, draft.token || undefined);
      setMessage(draft.key, { type: "ok", text: `✓ Conectado (account: ${userId})` });
    } catch (e) {
      setMessage(draft.key, { type: "error", text: String(e) });
    } finally {
      setValidatingKey(null);
    }
  }

  async function handleSave() {
    setSaving(true);
    setMessage("", null);
    try {
      const accounts: AccountInput[] = drafts.map((d) => ({
        id: d.id,
        kind: d.kind,
        workspace: d.workspace.trim(),
        repos: parseRepos(d.reposText),
        token: d.token || undefined,
      }));
      await saveConfig(pollingInterval, accounts);
      setMessage("", { type: "ok", text: "¡Guardado!" });
      setTimeout(onSaved, 600);
    } catch (e) {
      setMessage("", { type: "error", text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  const canSave = drafts.every(isDraftComplete);
  const formMessage = messages[""];

  return (
    <div className="settings">
      <span className="settings-section-title">Cuentas</span>

      {drafts.map((draft) => (
        <AccountForm
          key={draft.key}
          draft={draft}
          message={messages[draft.key] ?? null}
          validating={validatingKey === draft.key}
          onChange={(patch) => updateDraft(draft.key, patch)}
          onValidate={() => handleValidate(draft)}
          onRemove={() => setDrafts((ds) => ds.filter((d) => d.key !== draft.key))}
        />
      ))}

      <div>
        <button className="btn-secondary" onClick={() => setDrafts((ds) => [...ds, blankDraft()])}>
          + Añadir cuenta
        </button>
      </div>

      <div className="settings-field">
        <label>Polling (segundos)</label>
        <input
          type="number"
          value={pollingInterval}
          min={30}
          max={3600}
          onChange={(e) => setPollingInterval(Number(e.target.value))}
        />
      </div>

      {formMessage && (
        <p className={`settings-message settings-message--${formMessage.type}`}>
          {formMessage.text}
        </p>
      )}

      <div className="settings-actions">
        <div className="settings-actions-right">
          {onClose && (
            <button className="btn-secondary" onClick={onClose}>
              Cancelar
            </button>
          )}
          <button className="btn-primary" onClick={handleSave} disabled={saving || !canSave}>
            {saving ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </div>
    </div>
  );
}
