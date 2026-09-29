import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PullRequest } from "../types";

export function usePullRequests() {
  const [prs, setPrs] = useState<PullRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastFetched, setLastFetched] = useState<Date | null>(null);

  useEffect(() => {
    invoke<PullRequest[]>("get_pull_requests")
      .then((data) => {
        setPrs(data);
        setLoading(false);
        setLastFetched(new Date());
      })
      .catch((e) => {
        setError(String(e));
        setLoading(false);
      });

    const unlistenPrs = listen<PullRequest[]>("prs-updated", (event) => {
      setPrs(event.payload);
      setError(null);
      setLoading(false);
      setLastFetched(new Date());
    });

    const unlistenErr = listen<string>("poll-error", (event) => {
      setError(event.payload);
      setLoading(false);
    });

    return () => {
      unlistenPrs.then((fn) => fn());
      unlistenErr.then((fn) => fn());
    };
  }, []);

  function refresh() {
    setLoading(true);
    invoke("refresh_pull_requests").catch(console.error);
  }

  return { prs, loading, error, lastFetched, refresh };
}
