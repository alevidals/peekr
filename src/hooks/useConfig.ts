import { useCallback, useEffect, useRef, useState } from "react";
import { getConfig } from "../api";
import { applyAppearance } from "../theme";
import { DEFAULT_APPEARANCE, DEFAULT_NOTIFICATIONS, DEFAULT_REVIEW_QUEUE, type Config } from "../types";

const FALLBACK_CONFIG: Config = {
  polling_interval_secs: 120,
  theme: "system",
  appearance: DEFAULT_APPEARANCE,
  review_queue: DEFAULT_REVIEW_QUEUE,
  notifications: DEFAULT_NOTIFICATIONS,
  accounts: [],
};

/** `undefined` while loading. */
export function useConfig() {
  const [config, setConfig] = useState<Config | undefined>(undefined);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const reload = useCallback(() => {
    getConfig()
      .then((config) => {
        if (!mounted.current) return;
        applyAppearance(config.theme, config.appearance);
        setConfig(config);
      })
      .catch(() => {
        if (!mounted.current) return;
        applyAppearance(FALLBACK_CONFIG.theme, FALLBACK_CONFIG.appearance);
        setConfig(FALLBACK_CONFIG);
      });
  }, []);

  useEffect(reload, [reload]);

  const isConfigured = !!config?.accounts.some((a) => a.workspace !== "");
  return { config, isConfigured, reload };
}
