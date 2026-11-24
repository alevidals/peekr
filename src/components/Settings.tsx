import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { getConfig, saveConfig, validateCredentials } from "../api";
import { applyAppearance } from "../theme";
import {
  DEFAULT_APPEARANCE,
  DEFAULT_NOTIFICATIONS,
  DEFAULT_REVIEW_QUEUE,
  type Appearance,
  type Density,
  type NotificationSettings,
  type ReviewQueueSettings,
  type TextSize,
  type Theme,
} from "../types";
import { THEME_OPTIONS, type ThemeOption } from "../themes";
import { PROVIDERS } from "../providers";
import {
  blankDraft,
  configToDrafts,
  draftsToAccounts,
  isDraftComplete,
  type AccountDraft,
} from "../drafts";
import { IconAccount, IconInfo, IconPalette, IconPlus, IconSettings } from "./Icons";
import { AccountForm } from "./AccountForm";
import { AboutPanel } from "./AboutPanel";
import { ConfirmDialog } from "./ConfirmDialog";
import "./Settings.css";

interface Props {
  onSaved: () => void;
  onClose?: () => void;
}

const DENSITIES: { value: Density; label: string }[] = [
  { value: "comfortable", label: "Comfortable" },
  { value: "compact", label: "Compact" },
];

const TEXT_SIZES: { value: TextSize; label: string }[] = [
  { value: "small", label: "Small" },
  { value: "medium", label: "Medium" },
  { value: "large", label: "Large" },
];

type Section = "accounts" | "general" | "appearance" | "about";

const SECTIONS: {
  id: Section;
  label: string;
  description: string;
  icon: typeof IconSettings;
}[] = [
  {
    id: "accounts",
    label: "Accounts",
    description: "Connect your accounts and choose which repositories to watch.",
    icon: IconAccount,
  },
  {
    id: "general",
    label: "General",
    description: "Polling, startup, the review list and notifications.",
    icon: IconSettings,
  },
  {
    id: "appearance",
    label: "Appearance",
    description: "Theme, density and text size.",
    icon: IconPalette,
  },
  {
    id: "about",
    label: "About",
    description: "Version, paths and updates.",
    icon: IconInfo,
  },
];

const NOTIFICATION_TOGGLES: {
  key: keyof NotificationSettings;
  title: string;
  hint: string;
}[] = [
  {
    key: "new_review_request",
    title: "New review requests",
    hint: "When a pull request starts waiting on your review",
  },
  {
    key: "merge_ready",
    title: "Ready to merge",
    hint: "When one of your pull requests becomes mergeable",
  },
  {
    key: "checks_failed",
    title: "Failing checks",
    hint: "When checks or pipelines on your pull request fail",
  },
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
  const [reviewQueue, setReviewQueue] = useState<ReviewQueueSettings>(DEFAULT_REVIEW_QUEUE);
  const [notifications, setNotifications] =
    useState<NotificationSettings>(DEFAULT_NOTIFICATIONS);
  const [autostart, setAutostart] = useState(false);
  // What is currently stored, to revert the live preview on cancel and to know whether
  // the autostart switch changed.
  const saved = useRef({
    polling: String(DEFAULT_POLLING),
    theme: "system" as Theme,
    appearance: DEFAULT_APPEARANCE,
    reviewQueue: DEFAULT_REVIEW_QUEUE,
    notifications: DEFAULT_NOTIFICATIONS,
    autostart: false,
  });
  const [saving, setSaving] = useState(false);
  const [validatingKey, setValidatingKey] = useState<string | null>(null);
  // Per-account messages, keyed by draft key; "" is the form-level message.
  const [messages, setMessages] = useState<Record<string, Message | null>>({});
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [section, setSection] = useState<Section>("accounts");
  const [pendingRemoval, setPendingRemoval] = useState<AccountDraft | null>(null);
  const addAccountRef = useRef<HTMLButtonElement>(null);
  const previousAccountCount = useRef(0);

  useEffect(() => {
    let mounted = true;
    getConfig()
      .then((config) => {
        if (!mounted) return;
        setPollingText(String(config.polling_interval_secs));
        setTheme(config.theme);
        setAppearance(config.appearance);
        setReviewQueue(config.review_queue);
        setNotifications(config.notifications);
        saved.current.polling = String(config.polling_interval_secs);
        saved.current.theme = config.theme;
        saved.current.appearance = config.appearance;
        saved.current.reviewQueue = config.review_queue;
        saved.current.notifications = config.notifications;
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

  const requestRemoval = useCallback((draft: AccountDraft) => setPendingRemoval(draft), []);

  const confirmRemoval = useCallback(() => {
    if (!pendingRemoval) return;
    handleRemove(pendingRemoval.key);
    setPendingRemoval(null);
  }, [handleRemove, pendingRemoval]);

  // After a removal the trigger disappears; keep keyboard users in the account section.
  useEffect(() => {
    if (drafts.length < previousAccountCount.current) {
      addAccountRef.current?.focus();
    }
    previousAccountCount.current = drafts.length;
  }, [drafts.length]);

  const handleValidate = useCallback(
    async (draft: AccountDraft) => {
      setValidatingKey(draft.key);
      setMessage(draft.key, null);
      try {
        await validateCredentials(draft.kind, draft.id, draft.token || undefined);
        setMessage(draft.key, { type: "ok", text: "✓ Connected" });
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
  const patchAppearance = (patch: Partial<Appearance>) => changeLook(theme, { ...appearance, ...patch });
  // A named palette pins its own light/dark scheme; the default palette follows `theme`.
  const selectedTheme = THEME_OPTIONS.some((option) => option.value === appearance.palette)
    ? appearance.palette
    : theme;
  const chooseTheme = (option: ThemeOption) =>
    changeLook(option.theme, { ...appearance, palette: option.palette, accent: null });

  function handleClose() {
    if (savedTimer.current) {
      clearTimeout(savedTimer.current);
      savedTimer.current = null;
    }
    applyAppearance(saved.current.theme, saved.current.appearance);
    setPollingText(saved.current.polling);
    setTheme(saved.current.theme);
    setAppearance(saved.current.appearance);
    setReviewQueue(saved.current.reviewQueue);
    setNotifications(saved.current.notifications);
    setAutostart(saved.current.autostart);
    onClose?.();
  }

  async function handleSave() {
    setSaving(true);
    setMessage("", null);
    const polling = clampPolling(pollingText);
    try {
      await saveConfig(
        polling,
        theme,
        appearance,
        reviewQueue,
        notifications,
        draftsToAccounts(drafts)
      );
      setPollingText(String(polling));
      saved.current.polling = String(polling);
      saved.current.theme = theme;
      saved.current.appearance = appearance;
      saved.current.reviewQueue = reviewQueue;
      saved.current.notifications = notifications;
      if (autostart !== saved.current.autostart) {
        await (autostart ? enable() : disable());
        saved.current.autostart = autostart;
      }
      setMessage("", { type: "ok", text: "Saved!" });
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
  const currentSection = SECTIONS.find((item) => item.id === section) ?? SECTIONS[0];
  const removingSaved = pendingRemoval?.id != null;
  const removalName = pendingRemoval?.workspace.trim() || "this account";
  const removalProvider = pendingRemoval ? PROVIDERS[pendingRemoval.kind].label : "";

  function onNavKeyDown(e: React.KeyboardEvent<HTMLButtonElement>, index: number) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    e.preventDefault();
    let next = index;
    if (e.key === "ArrowDown") next = (index + 1) % SECTIONS.length;
    if (e.key === "ArrowUp") next = (index - 1 + SECTIONS.length) % SECTIONS.length;
    if (e.key === "Home") next = 0;
    if (e.key === "End") next = SECTIONS.length - 1;
    const id = SECTIONS[next].id;
    setSection(id);
    document.getElementById(`settings-tab-${id}`)?.focus();
  }

  return (
    <div className="settings" inert={pendingRemoval !== null}>
      <header className="settings-header" data-tauri-drag-region>
        <div className="settings-brand">
          <img className="settings-logo" src="/app-icon.png" alt="" />
          <span className="settings-wordmark">peekr</span>
        </div>
        <span className="settings-header-divider" aria-hidden="true" />
        <div className="settings-header-text">
          <h1>Settings</h1>
          <p>Accounts, preferences and appearance.</p>
        </div>
      </header>

      <div className="settings-body">
        <aside className="settings-sidebar">
          <div
            className="settings-nav"
            role="tablist"
            aria-orientation="vertical"
            aria-label="Settings sections"
          >
            {SECTIONS.map((item, index) => {
              const Icon = item.icon;
              const selected = section === item.id;
              return (
                <button
                  key={item.id}
                  id={`settings-tab-${item.id}`}
                  type="button"
                  role="tab"
                  aria-selected={selected}
                  aria-controls="settings-panel"
                  aria-label={item.id === "accounts" ? `Accounts, ${drafts.length}` : undefined}
                  tabIndex={selected ? 0 : -1}
                  className={`settings-nav-item${item.id === "about" ? " settings-nav-item--about" : ""}`}
                  onClick={() => setSection(item.id)}
                  onKeyDown={(e) => onNavKeyDown(e, index)}
                >
                  <Icon size={15} className="settings-nav-icon" />
                  <span>{item.label}</span>
                  {item.id === "accounts" && (
                    <span className="settings-nav-count">{drafts.length}</span>
                  )}
                </button>
              );
            })}
          </div>

          <div className="settings-sidebar-footer">
            Made with ♥ by{" "}
            <button type="button" onClick={() => openUrl("https://github.com/alevidals")}>
              alevidals
            </button>
          </div>
        </aside>

        <main className="settings-main">
          <div
            className="settings-content"
            id="settings-panel"
            role="tabpanel"
            aria-labelledby={`settings-tab-${section}`}
          >
            {section !== "about" && (
              <div className="settings-section-head">
                <div>
                  <h2>{currentSection.label}</h2>
                  <p>{currentSection.description}</p>
                </div>
                {section === "appearance" && (
                  <button
                    type="button"
                    className="link-btn"
                    onClick={() => changeLook("system", DEFAULT_APPEARANCE)}
                  >
                    Reset
                  </button>
                )}
              </div>
            )}

            {section === "accounts" && (
              <div className="settings-stack">
                {drafts.map((draft) => (
                  <AccountForm
                    key={draft.key}
                    draft={draft}
                    defaultOpen={drafts.length === 1 || !isDraftComplete(draft)}
                    message={messages[draft.key] ?? null}
                    validating={validatingKey === draft.key}
                    onChange={handleDraftChange}
                    onValidate={handleValidate}
                    onRequestRemove={requestRemoval}
                  />
                ))}

                <button
                  ref={addAccountRef}
                  type="button"
                  className="add-account"
                  onClick={() => setDrafts((ds) => [...ds, blankDraft()])}
                >
                  <IconPlus size={14} /> Add account
                </button>
              </div>
            )}

            {section === "general" && (
              <div className="settings-stack">
                <p className="settings-group-title">Polling</p>
                <div className="card row">
                  <div className="row-text">
                    <span className="row-title" id="polling-label">Polling interval</span>
                    <span className="hint">How often new PRs are fetched (30–3600 s)</span>
                  </div>
                  <div className="input-suffix">
                    <input
                      type="number"
                      aria-labelledby="polling-label"
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

                <p className="settings-group-title">Startup</p>
                <div className="card row">
                  <div className="row-text">
                    <span className="row-title">Launch at login</span>
                    <span className="hint">Open peekr automatically when you log in</span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={autostart}
                    aria-label="Launch at login"
                    className={`switch${autostart ? " switch--on" : ""}`}
                    onClick={() => setAutostart((v) => !v)}
                  />
                </div>

                <p className="settings-group-title">Review list</p>
                <div className="card row">
                  <div className="row-text">
                    <span className="row-title">Show approved pull requests</span>
                    <span className="hint">
                      Keep the pull requests you already approved in the review tab. They never
                      count towards the tray badge or new-request notifications.
                    </span>
                  </div>
                  <button
                    type="button"
                    role="switch"
                    aria-checked={reviewQueue.show_approved}
                    aria-label="Show approved pull requests"
                    className={`switch${reviewQueue.show_approved ? " switch--on" : ""}`}
                    onClick={() =>
                      setReviewQueue((current) => ({
                        ...current,
                        show_approved: !current.show_approved,
                      }))
                    }
                  />
                </div>

                <p className="settings-group-title">Notifications</p>
                {NOTIFICATION_TOGGLES.map(({ key, title, hint }) => (
                  <div className="card row" key={key}>
                    <div className="row-text">
                      <span className="row-title">{title}</span>
                      <span className="hint">{hint}</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={notifications[key]}
                      aria-label={title}
                      className={`switch${notifications[key] ? " switch--on" : ""}`}
                      onClick={() =>
                        setNotifications((current) => ({
                          ...current,
                          [key]: !current[key],
                        }))
                      }
                    />
                  </div>
                ))}
              </div>
            )}

            {section === "appearance" && (
              <div className="settings-stack">
                <div className="theme-grid" role="radiogroup" aria-label="Theme">
                  {THEME_OPTIONS.map((option) => {
                    const selected = selectedTheme === option.value;
                    return (
                      <label
                        key={option.value}
                        className={`theme-card${selected ? " theme-card--active" : ""}`}
                        title={option.description}
                      >
                        <input
                          type="radio"
                          name="theme"
                          value={option.value}
                          checked={selected}
                          onChange={() => chooseTheme(option)}
                          className="sr-only"
                        />
                        <span
                          className="theme-preview"
                          style={{ background: option.preview.background }}
                          aria-hidden="true"
                        >
                          <span className="theme-preview-dot" style={{ background: option.preview.accent }} />
                          <span className="theme-preview-line" style={{ background: option.preview.foreground }} />
                        </span>
                        <span className="theme-label">{option.label}</span>
                        <span className="theme-description">{option.description}</span>
                      </label>
                    );
                  })}
                </div>

                <div className="card row">
                  <div className="row-text">
                    <span className="row-title" id="density-label">Density</span>
                    <span className="hint">How much space each PR takes</span>
                  </div>
                  <div className="segmented segmented--compact" role="radiogroup" aria-labelledby="density-label">
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
                    <span className="row-title" id="text-size-label">Text size</span>
                    <span className="hint">In the PR list</span>
                  </div>
                  <div className="segmented segmented--compact" role="radiogroup" aria-labelledby="text-size-label">
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
            )}

            {section === "about" && <AboutPanel />}
          </div>
        </main>
      </div>

      <footer className="settings-footer">
        {formMessage ? (
          <p
            className={`message message--${formMessage.type}`}
            role={formMessage.type === "error" ? "alert" : "status"}
          >
            {formMessage.text}
          </p>
        ) : (
          <span />
        )}
        <div className="settings-footer-actions">
          {onClose && (
            <button className="btn btn--ghost" onClick={handleClose}>
              Cancel
            </button>
          )}
          <button className="btn btn--primary" onClick={handleSave} disabled={saving || !canSave}>
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </footer>

      <ConfirmDialog
        open={pendingRemoval !== null}
        title={removingSaved ? "Remove account?" : "Discard account?"}
        description={
          removingSaved ? (
            <>
              Saving will remove <strong>{removalName}</strong> from {removalProvider} and
              delete its stored token from the keychain. This cannot be undone.
            </>
          ) : (
            "This account has not been saved yet. Discarding it will lose the workspace, token and repository details you entered."
          )
        }
        confirmLabel={removingSaved ? "Remove account" : "Discard account"}
        cancelLabel={removingSaved ? "Keep account" : "Keep editing"}
        onConfirm={confirmRemoval}
        onCancel={() => setPendingRemoval(null)}
      />
    </div>
  );
}
