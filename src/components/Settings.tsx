import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Config } from "../types";

interface Props {
  onSaved: () => void;
  onClose?: () => void;
}

export function Settings({ onSaved, onClose }: Props) {
  const [workspace, setWorkspace] = useState("");
  const [token, setToken] = useState("");
  const [hasToken, setHasToken] = useState(false);
  const [pollingInterval, setPollingInterval] = useState(120);
  const [reposText, setReposText] = useState("");
  const [saving, setSaving] = useState(false);
  const [validating, setValidating] = useState(false);
  const [message, setMessage] = useState<{ type: "ok" | "error"; text: string } | null>(null);

  useEffect(() => {
    invoke<Config | null>("get_config").then((config) => {
      if (config) {
        setWorkspace(config.workspace);
        setPollingInterval(config.polling_interval_secs);
        setReposText(config.repos.join("\n"));
        setHasToken(config.has_token);
      }
    });
  }, []);

  const repos = reposText
    .split("\n")
    .map((r) => r.trim())
    .filter(Boolean);

  async function handleValidate() {
    if (!token) {
      setMessage({ type: "error", text: "Introduce el token primero." });
      return;
    }
    setValidating(true);
    setMessage(null);
    try {
      const accountId = await invoke<string>("validate_credentials", { token });
      setMessage({ type: "ok", text: `✓ Conectado (account: ${accountId})` });
    } catch (e) {
      setMessage({ type: "error", text: String(e) });
    } finally {
      setValidating(false);
    }
  }

  async function handleSave() {
    if (!workspace || (!token && !hasToken) || repos.length === 0) {
      setMessage({ type: "error", text: "Rellena workspace, token y al menos un repo." });
      return;
    }
    setSaving(true);
    setMessage(null);
    try {
      await invoke("save_credentials", {
        workspace,
        token,
        repos,
        pollingIntervalSecs: pollingInterval,
      });
      setMessage({ type: "ok", text: "¡Guardado!" });
      invoke("refresh_pull_requests").catch(console.error);
      setTimeout(onSaved, 600);
    } catch (e) {
      setMessage({ type: "error", text: String(e) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="settings">
      <div className="settings-field">
        <label>Workspace</label>
        <input
          type="text"
          value={workspace}
          onChange={(e) => setWorkspace(e.target.value)}
          placeholder="my-workspace"
        />
        <span className="settings-hint">El slug de la URL: bitbucket.org/<strong>workspace</strong>/repo</span>
      </div>

      <div className="settings-field">
        <label>Token</label>
        <input
          type="password"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder={hasToken ? "dejar en blanco para no cambiar" : "••••••••••••"}
        />
        <span className="settings-hint">Repository o Workspace Access Token de Bitbucket Cloud</span>
      </div>

      <div className="settings-field">
        <label>Repos a monitorizar</label>
        <textarea
          value={reposText}
          onChange={(e) => setReposText(e.target.value)}
          placeholder={"my-repo\nanother-repo\nthird-repo"}
          rows={5}
          spellCheck={false}
        />
        <span className="settings-hint">Un slug por línea — el nombre que aparece en la URL</span>
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

      {message && (
        <p className={`settings-message settings-message--${message.type}`}>
          {message.text}
        </p>
      )}

      <div className="settings-actions">
        <button className="btn-secondary" onClick={handleValidate} disabled={validating}>
          {validating ? "Comprobando…" : "Validar token"}
        </button>
        <div className="settings-actions-right">
          {onClose && (
            <button className="btn-secondary" onClick={onClose}>
              Cancelar
            </button>
          )}
          <button
            className="btn-primary"
            onClick={handleSave}
            disabled={saving || !workspace || (!token && !hasToken) || repos.length === 0}
          >
            {saving ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </div>
    </div>
  );
}
