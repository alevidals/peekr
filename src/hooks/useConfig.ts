import { useCallback, useEffect, useState } from "react";
import { getConfig } from "../api";
import type { Config } from "../types";

/** `undefined` while loading. */
export function useConfig() {
  const [config, setConfig] = useState<Config | undefined>(undefined);

  const reload = useCallback(() => {
    getConfig()
      .then(setConfig)
      .catch(() => setConfig({ polling_interval_secs: 120, accounts: [] }));
  }, []);

  useEffect(reload, [reload]);

  const isConfigured = !!config?.accounts.some((a) => a.workspace !== "");
  return { config, isConfigured, reload };
}
