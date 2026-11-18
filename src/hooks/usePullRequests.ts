import { useCallback, useEffect, useState } from "react";
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
    let mounted = true;

    getPullRequests()
      .then((data) => {
        if (!mounted) return;
        setPrs(data);
        setLastFetched(new Date());
      })
      .catch((e) => {
        if (mounted) setError(String(e));
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });

    getMyPullRequests()
      .then((data) => {
        if (mounted) setMine(data);
      })
      .catch((e) => {
        if (mounted) setError(String(e));
      });

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
      mounted = false;
      unlistenMine.then((fn) => fn());
      unlistenPrs.then((fn) => fn());
      unlistenErr.then((fn) => fn());
    };
  }, []);

  const refresh = useCallback(() => {
    setLoading(true);
    refreshPullRequests().catch((e) => {
      setError(String(e));
      setLoading(false);
    });
  }, []);

  return { prs, mine, loading, error, lastFetched, refresh };
}
