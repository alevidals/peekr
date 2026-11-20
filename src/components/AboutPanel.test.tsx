import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getAppInfo } from "../api";
import { AboutPanel } from "./AboutPanel";

vi.mock("../api", () => ({
  getAppInfo: vi.fn(),
  openLogs: vi.fn(),
  checkForUpdates: vi.fn(),
}));

const mockedInfo = vi.mocked(getAppInfo);

describe("AboutPanel", () => {
  beforeEach(() => {
    mockedInfo.mockResolvedValue({
      version: "0.1.0",
      platform: "macos",
      arch: "aarch64",
      config_dir: "/Users/test/Library/Application Support/com.alevidals.peekr",
      log_dir: "/Users/test/Library/Logs/com.alevidals.peekr",
    });
  });

  it("shows the app metadata and the available actions", async () => {
    render(<AboutPanel />);

    expect(await screen.findByText("0.1.0")).toBeInTheDocument();
    expect(screen.getByText("macos / aarch64")).toBeInTheDocument();
    expect(screen.getByText(/Application Support\/com\.alevidals\.peekr/)).toBeInTheDocument();
    expect(screen.getByText(/Library\/Logs\/com\.alevidals\.peekr/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open logs" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check for updates" })).toBeInTheDocument();
  });
});
