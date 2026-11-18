import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { getConfig, saveConfig, validateCredentials } from "../api";
import { applyAppearance, PALETTES, paletteAccent } from "../theme";
import {
  DEFAULT_APPEARANCE,
  type Appearance,
  type Density,
  type Palette,
  type TextSize,
  type Theme,
} from "../types";
import {
  blankDraft,
  configToDrafts,
  draftsToAccounts,
  isDraftComplete,
  type AccountDraft,
} from "../drafts";
import { IconPlus } from "./Icons";
import { AccountForm } from "./AccountForm";
import "./Settings.css";

interface Props {
  onSaved: () => void;
  onClose?: () => void;
}

const THEMES: { value: Theme; label: string }[] = [
  { value: "system", label: "Sistema" },
  { value: "light", label: "Claro" },
  { value: "dark", label: "Oscuro" },
];

const DENSITIES: { value: Density; label: string }[] = [
  { value: "comfortable", label: "Cómoda" },
  { value: "compact", label: "Compacta" },
];

const TEXT_SIZES: { value: TextSize; label: string }[] = [
  { value: "small", label: "Pequeño" },
  { value: "medium", label: "Medio" },
  { value: "large", label: "Grande" },
];

const MIN_POLLING = 30;
const MAX_POLLING = 3600;
const DEFAULT_POLLING = 120;

/** Keeps an out-of-range or empty text field valid without fighting what the user typed. */
function clampPolling(value: string): number {
  const secs = Math.round(Number(value));
  if (!Number.isFinite(secs)) return DEFAULT_POLLING;
  return Math.min(MAX_POLLING, Math.max(MIN_POLLING, secs));
}

type Message = { type: "ok" | "error"; text: string };

export function Settings({ onSaved, onClose }: Props) {
  const [drafts, setDrafts] = useState<AccountDraft[]>([]);
  const [pollingText, setPollingText] = useState(String(DEFAULT_POLLING));
  const [theme, setTheme] = useState<Theme>("system");
  const [appearance, setAppearance] = useState<Appearance>(DEFAULT_APPEARANCE);
  const [autostart, setAutostart] = useState(false);
  // What is currently stored, to revert the live appearance preview on cancel and to know
  // whether the autostart switch changed.
  const saved = useRef({ theme: "system" as Theme, appearance: DEFAULT_APPEARANCE, autostart: false });
  const [saving, setSaving] = useState(false);
  const [validatingKey, setValidatingKey] = useState<string | null>(null);
  // Per-account messages, keyed by draft key; "" is the form-level message.
  const [messages, setMessages] = useState<Record<string, Message | null>>({});
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let mounted = true;
    getConfig()
      .then((config) => {
        if (!mounted) return;
        setPollingText(String(config.polling_interval_secs));
        setTheme(config.theme);
        setAppearance(config.appearance);
        saved.current.theme = config.theme;
        saved.current.appearance = config.appearance;
        applyAppearance(config.theme, config.appearance);
        setDrafts(configToDrafts(config));
      })
      .catch(console.error);
    // Autostart is registered with the OS, not stored in our config.
    isEnabled()
      .then((enabled) => {
        if (!mounted) return;
        saved.current.autostart = enabled;
        setAutostart(enabled);
      })
      .catch(console.error);

    return () => {
      mounted = false;
      if (savedTimer.current) clearTimeout(savedTimer.current);
    };
  }, []);

  const setMessage = useCallback(
    (key: string, message: Message | null) =>
      setMessages((m) => ({ ...m, [key]: message })),
    []
  );

  const handleDraftChange = useCallback(
    (key: string, patch: Partial<AccountDraft>) => {
      // Created outside the updater so the updater stays pure (StrictMode runs it twice).
      const reset = patch.kind ? blankDraft(patch.kind) : null;
      setDrafts((ds) =>
        ds.map((d) => {
          if (d.key !== key) return d;
          // Workspace, token and repos mean nothing for another provider: start a fresh
          // account (no id, so the old one and its stored token are dropped on save).
          if (reset && patch.kind !== d.kind) return { ...reset, key: d.key };
          return { ...d, ...patch };
        })
      );
      if (patch.kind) setMessage(key, null);
    },
    [setMessage]
  );

  const handleRemove = useCallback((key: string) => {
    setDrafts((ds) => ds.filter((d) => d.key !== key));
    setMessages((m) => {
      const next = { ...m };
      delete next[key];
      return next;
    });
  }, []);

  const handleValidate = useCallback(
    async (draft: AccountDraft) => {
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
    },
    [setMessage]
  );

  // Live preview; reverted on cancel.
  function changeLook(nextTheme: Theme, nextAppearance: Appearance) {
    setTheme(nextTheme);
    setAppearance(nextAppearance);
    applyAppearance(nextTheme, nextAppearance);
  }
  const changeTheme = (next: Theme) => changeLook(next, appearance);
  const patchAppearance = (patch: Partial<Appearance>) => changeLook(theme, { ...appearance, ...patch });

  function handleClose() {
    if (savedTimer.current) {
      clearTimeout(savedTimer.current);
      savedTimer.current = null;
    }
    applyAppearance(saved.current.theme, saved.current.appearance);
    setTheme(saved.current.theme);
    setAppearance(saved.current.appearance);
    setAutostart(saved.current.autostart);
    onClose?.();
  }

  async function handleSave() {
    setSaving(true);
    setMessage("", null);
    const polling = clampPolling(pollingText);
    try {
      await saveConfig(polling, theme, appearance, draftsToAccounts(drafts));
      setPollingText(String(polling));
      saved.current.theme = theme;
      saved.current.appearance = appearance;
      if (autostart !== saved.current.autostart) {
        await (autostart ? enable() : disable());
        saved.current.autostart = autostart;
      }
      setMessage("", { type: "ok", text: "¡Guardado!" });
      if (savedTimer.current) clearTimeout(savedTimer.current);
      savedTimer.current = setTimeout(onSaved, 600);
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
              onChange={handleDraftChange}
              onValidate={handleValidate}
              onRemove={handleRemove}
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
                value={pollingText}
                min={MIN_POLLING}
                max={MAX_POLLING}
                inputMode="numeric"
                onChange={(e) => setPollingText(e.target.value)}
                onBlur={() => setPollingText(String(clampPolling(pollingText)))}
              />
              <span>s</span>
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

        <div className="section">
          <div className="section-head">
            <h2>Apariencia</h2>
            <button
              type="button"
              className="link-btn"
              onClick={() => changeLook("system", DEFAULT_APPEARANCE)}
            >
              Restablecer
            </button>
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
              <span className="row-title">Paleta</span>
              <span className="hint">Colores base de peekr</span>
            </div>
            <div className="swatches" role="radiogroup">
              {(Object.keys(PALETTES) as Palette[]).map((palette) => (
                <button
                  key={palette}
                  type="button"
                  role="radio"
                  aria-checked={appearance.palette === palette}
                  aria-label={PALETTES[palette].label}
                  title={PALETTES[palette].label}
                  className={`swatch${appearance.palette === palette ? " swatch--active" : ""}`}
                  style={{ background: paletteAccent(palette, theme) }}
                  onClick={() => patchAppearance({ palette, accent: null })}
                />
              ))}
            </div>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Color de acento</span>
              <span className="hint">Sustituye al de la paleta</span>
            </div>
            <div className="swatches">
              {appearance.accent && (
                <button type="button" className="link-btn" onClick={() => patchAppearance({ accent: null })}>
                  Quitar
                </button>
              )}
              <input
                type="color"
                className="accent-input"
                aria-label="Color de acento"
                value={appearance.accent ?? paletteAccent(appearance.palette, theme)}
                onChange={(e) => patchAppearance({ accent: e.target.value })}
              />
            </div>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Densidad</span>
              <span className="hint">Cuánto espacio ocupa cada PR</span>
            </div>
            <div className="segmented segmented--compact" role="radiogroup">
              {DENSITIES.map(({ value, label }) => (
                <button
                  key={value}
                  type="button"
                  role="radio"
                  aria-checked={appearance.density === value}
                  className={`segmented-item${appearance.density === value ? " segmented-item--active" : ""}`}
                  onClick={() => patchAppearance({ density: value })}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <div className="card row">
            <div className="row-text">
              <span className="row-title">Tamaño del texto</span>
              <span className="hint">En la lista de PRs</span>
            </div>
            <div className="segmented segmented--compact" role="radiogroup">
              {TEXT_SIZES.map(({ value, label }) => (
                <button
                  key={value}
                  type="button"
                  role="radio"
                  aria-checked={appearance.text_size === value}
                  className={`segmented-item${appearance.text_size === value ? " segmented-item--active" : ""}`}
                  onClick={() => patchAppearance({ text_size: value })}
                >
                  {label}
                </button>
              ))}
            </div>
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
