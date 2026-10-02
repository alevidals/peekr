import { useCallback, useEffect, useState } from "react";
import { getConfig } from "../api";
import { applyAppearance } from "../theme";
import { DEFAULT_APPEARANCE, type Config } from "../types";

/** `undefined` while loading. */
export function useConfig() {
  const [config, setConfig] = useState<Config | undefined>(undefined);

  const reload = useCallback(() => {
    getConfig()
      .then((config) => {
        applyAppearance(config.theme, config.appearance);
        setConfig(config);
      })
      .catch(() => setConfig({ polling_interval_secs: 120, theme: "system", appearance: DEFAULT_APPEARANCE, accounts: [] }));
  }, []);

  useEffect(reload, [reload]);

  const isConfigured = !!config?.accounts.some((a) => a.workspace !== "");
  return { config, isConfigured, reload };
}
