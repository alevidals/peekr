import { useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { getConfig, saveConfig, validateCredentials } from "../api";
import { applyTheme } from "../theme";
import type { AccountInput, Theme } from "../types";
import { IconPlus } from "./Icons";
import { AccountForm, isDraftComplete, parseRepos, type AccountDraft } from "./AccountForm";

interface Props {
  onSaved: () => void;
  onClose?: () => void;
}

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "Sistema" },
  { value: "light", label: "Claro" },
  { value: "dark", label: "Oscuro" },
];

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
  const [theme, setTheme] = useState<Theme>("system");
  const [autostart, setAutostart] = useState(false);
  // What is currently stored, to revert the live theme preview on cancel and to know
  // whether the autostart switch changed.
  const saved = useRef({ theme: "system" as Theme, autostart: false });
  const [saving, setSaving] = useState(false);
  const [validatingKey, setValidatingKey] = useState<string | null>(null);
  // Per-account messages, keyed by draft key; "" is the form-level message.
  const [messages, setMessages] = useState<Record<string, Message | null>>({});

  useEffect(() => {
    getConfig().then((config) => {
      setPollingInterval(config.polling_interval_secs);
      setTheme(config.theme);
      saved.current.theme = config.theme;
      applyTheme(config.theme);
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
    // Autostart is registered with the OS, not stored in our config.
    isEnabled()
      .then((enabled) => {
        saved.current.autostart = enabled;
        setAutostart(enabled);
      })
      .catch(console.error);
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

  function changeTheme(next: Theme) {
    setTheme(next);
    applyTheme(next); // live preview; reverted on cancel
  }

  function handleClose() {
    applyTheme(saved.current.theme);
    setTheme(saved.current.theme);
    setAutostart(saved.current.autostart);
    onClose?.();
  }

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
      await saveConfig(pollingInterval, theme, accounts);
      saved.current.theme = theme;
      if (autostart !== saved.current.autostart) {
        await (autostart ? enable() : disable());
        saved.current.autostart = autostart;
      }
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
      <div className="settings-body">
        <header className="settings-hero">
          <h1>Ajustes</h1>
          <p>Conecta tus cuentas y elige qué repositorios vigilar.</p>
        </header>

        <div className="section">
          <div className="section-head">
            <h2>Cuentas</h2>
            <span className="section-count">{drafts.length}</span>
          </div>

          {drafts.map((draft) => (
            <AccountForm
              key={draft.key}
              draft={draft}
              defaultOpen={drafts.length === 1 || !isDraftComplete(draft)}
              message={messages[draft.key] ?? null}
              validating={validatingKey === draft.key}
              onChange={(patch) => updateDraft(draft.key, patch)}
              onValidate={() => handleValidate(draft)}
              onRemove={() => setDrafts((ds) => ds.filter((d) => d.key !== draft.key))}
            />
          ))}

          <button
            type="button"
            className="add-account"
            onClick={() => setDrafts((ds) => [...ds, blankDraft()])}
          >
            <IconPlus size={14} /> Añadir cuenta
          </button>
        </div>

        <div className="section">
          <div className="section-head">
            <h2>General</h2>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Frecuencia de consulta</span>
              <span className="hint">Cada cuánto se buscan PRs nuevas (30–3600 s)</span>
            </div>
            <div className="input-suffix">
              <input
                type="number"
                value={pollingInterval}
                min={30}
                max={3600}
                onChange={(e) => setPollingInterval(Number(e.target.value))}
              />
              <span>s</span>
            </div>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Tema</span>
              <span className="hint">Apariencia de peekr</span>
            </div>
            <div className="segmented segmented--compact" role="radiogroup">
              {THEMES.map(({ value, label }) => (
                <button
                  key={value}
                  type="button"
                  role="radio"
                  aria-checked={theme === value}
                  className={`segmented-item${theme === value ? " segmented-item--active" : ""}`}
                  onClick={() => changeTheme(value)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Iniciar con el sistema</span>
              <span className="hint">Abre peekr automáticamente al iniciar sesión</span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={autostart}
              aria-label="Iniciar con el sistema"
              className={`switch${autostart ? " switch--on" : ""}`}
              onClick={() => setAutostart((v) => !v)}
            />
          </div>
        </div>

        <p className="settings-credit">
          Hecho con ♥ por{" "}
          <button type="button" onClick={() => openUrl("https://github.com/alevidals")}>
            alevidals
          </button>
        </p>
      </div>

      <footer className="settings-footer">
        {formMessage ? (
          <p className={`message message--${formMessage.type}`}>{formMessage.text}</p>
        ) : (
          <span />
        )}
        <div className="settings-footer-actions">
          {onClose && (
            <button className="btn btn--ghost" onClick={handleClose}>
              Cancelar
            </button>
          )}
          <button className="btn btn--primary" onClick={handleSave} disabled={saving || !canSave}>
            {saving ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </footer>
    </div>
  );
}
