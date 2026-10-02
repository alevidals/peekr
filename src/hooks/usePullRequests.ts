import { useEffect, useState } from "react";
import {
  getMyPullRequests,
  getPullRequests,
  onMyPrsUpdated,
  onPollError,
  onPrsUpdated,
  refreshPullRequests,
} from "../api";
import type { PullRequest } from "../types";

export function usePullRequests() {
  const [prs, setPrs] = useState<PullRequest[]>([]);
  const [mine, setMine] = useState<PullRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastFetched, setLastFetched] = useState<Date | null>(null);

  useEffect(() => {
    getPullRequests()
      .then((data) => {
        setPrs(data);
        setLastFetched(new Date());
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));

    getMyPullRequests().then(setMine).catch(console.error);
    const unlistenMine = onMyPrsUpdated(setMine);

    const unlistenPrs = onPrsUpdated((data) => {
      setPrs(data);
      setError(null);
      setLoading(false);
      setLastFetched(new Date());
    });

    const unlistenErr = onPollError((message) => {
      setError(message);
      setLoading(false);
    });

    return () => {
      unlistenMine.then((fn) => fn());
      unlistenPrs.then((fn) => fn());
      unlistenErr.then((fn) => fn());
    };
  }, []);

  function refresh() {
    setLoading(true);
    refreshPullRequests().catch(console.error);
  }

  return { prs, mine, loading, error, lastFetched, refresh };
}
