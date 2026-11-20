import { useEffect, useState } from "react";
import { getAppInfo, openLogs } from "../api";
import type { AppInfo } from "../types";
import { UpdateCheck } from "./UpdateCheck";

/** Settings → About: identity, paths, credential note and the manual update check. */
export function AboutPanel() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    getAppInfo()
      .then((value) => {
        if (mounted) setInfo(value);
      })
      .catch((cause) => {
        if (mounted) setError(String(cause));
      });
    return () => {
      mounted = false;
    };
  }, []);

  return (
    <div className="about">
      <div className="about-brand">
        <img className="about-logo" src="/app-icon.png" alt="" />
        <div>
          <h3>peekr</h3>
          <p>A tray client for the pull requests waiting on your review.</p>
        </div>
      </div>

      <dl className="about-grid">
        <dt>Version</dt>
        <dd>{info?.version ?? "—"}</dd>
        <dt>Platform</dt>
        <dd>{info ? `${info.platform} / ${info.arch}` : "—"}</dd>
        <dt>Config</dt>
        <dd className="about-path">{info?.config_dir ?? "—"}</dd>
        <dt>Logs</dt>
        <dd className="about-path">{info?.log_dir ?? "—"}</dd>
      </dl>

      {error ? (
        <p className="message message--error about-message" role="alert">
          {error}
        </p>
      ) : null}

      <p className="about-note">
        Credentials are stored in the operating system keychain and never written to
        configuration files or logs.
      </p>

      <div className="about-actions">
        <button
          type="button"
          className="btn btn--secondary"
          onClick={() => {
            openLogs().catch((cause) => setError(String(cause)));
          }}
        >
          Open logs
        </button>
      </div>

      <p className="about-credit">
        Set in Instrument Serif, DM Sans and IBM Plex Mono, licensed under the SIL OFL 1.1.
      </p>

      <UpdateCheck />
    </div>
  );
}
