import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { checkForUpdates } from "../api";
import type { UpdateStatus } from "../types";
import { IconCheck, IconRefresh } from "./Icons";

type Phase = "idle" | "checking" | "done";

/** Manual update check. One request, display-only; the release page opens in the browser. */
export function UpdateCheck() {
  const [phase, setPhase] = useState<Phase>("idle");
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function check() {
    setPhase("checking");
    setError(null);
    try {
      setStatus(await checkForUpdates());
      setPhase("done");
    } catch (cause) {
      setStatus(null);
      setError(String(cause));
      setPhase("idle");
    }
  }

  return (
    <div className="card update-check">
      <div className="row">
        <div className="row-text">
          <span className="row-title">Updates</span>
          <span className="hint">
            Checks the public releases page once. Nothing is downloaded automatically.
          </span>
        </div>
        <button
          type="button"
          className="btn btn--secondary"
          onClick={() => void check()}
          disabled={phase === "checking"}
        >
          {phase === "checking" ? (
            "Checking…"
          ) : (
            <>
              <IconRefresh size={13} /> Check for updates
            </>
          )}
        </button>
      </div>

      {error ? (
        <p className="message message--error update-check-message" role="alert">
          {error}
        </p>
      ) : null}

      {phase === "done" && status ? (
        status.update_available ? (
          <div className="update-check-result" role="status">
            <span>
              peekr <code>{status.latest_version}</code> is available. You have{" "}
              <code>{status.current_version}</code>.
            </span>
            <button
              type="button"
              className="btn btn--secondary"
              onClick={() => void openUrl(status.release_url)}
            >
              Release page
            </button>
          </div>
        ) : (
          <p className="update-check-latest" role="status">
            <IconCheck size={12} /> You are on the latest version ({status.current_version}).
          </p>
        )
      ) : null}
    </div>
  );
}
