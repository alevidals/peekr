// Single place that knows the names of the Tauri commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AccountInput,
  Appearance,
  AppInfo,
  Config,
  PopoverPlacement,
  ProviderKind,
  PullRequest,
  Theme,
  UpdateStatus,
} from "./types";

export const getAppInfo = () => invoke<AppInfo>("app_info");
export const getConfig = () => invoke<Config>("get_config");

export const saveConfig = (
  pollingIntervalSecs: number,
  theme: Theme,
  appearance: Appearance,
  accounts: AccountInput[]
) => invoke<void>("save_config", { pollingIntervalSecs, theme, appearance, accounts });

export const validateCredentials = (kind: ProviderKind, accountId?: string, token?: string) =>
  invoke<string>("validate_credentials", { kind, accountId, token });

export const getPullRequests = () => invoke<PullRequest[]>("get_pull_requests");
export const getMyPullRequests = () => invoke<PullRequest[]>("get_my_pull_requests");
export const refreshPullRequests = () => invoke<void>("refresh_pull_requests");
export const openSettingsWindow = () => invoke<void>("open_settings_window");
export const hideSettingsWindow = () => invoke<void>("hide_settings_window");
export const hidePopover = () => invoke<void>("hide_popover");
export const openLogs = () => invoke<void>("open_logs");
export const checkForUpdates = () => invoke<UpdateStatus>("check_for_updates");

export const onPrsUpdated = (cb: (prs: PullRequest[]) => void): Promise<UnlistenFn> =>
  listen<PullRequest[]>("prs-updated", (e) => cb(e.payload));

export const onMyPrsUpdated = (cb: (prs: PullRequest[]) => void): Promise<UnlistenFn> =>
  listen<PullRequest[]>("my-prs-updated", (e) => cb(e.payload));

export const onPollError = (cb: (message: string) => void): Promise<UnlistenFn> =>
  listen<string>("poll-error", (e) => cb(e.payload));

export const onPopoverWillHide = (cb: () => void): Promise<UnlistenFn> =>
  listen("popover-will-hide", () => cb());

export const onPopoverWillShow = (cb: (placement: PopoverPlacement) => void): Promise<UnlistenFn> =>
  listen<PopoverPlacement>("popover-will-show", (e) => cb(e.payload));
