import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { checkForUpdates } from "../api";
import { UpdateCheck } from "./UpdateCheck";

vi.mock("../api", () => ({ checkForUpdates: vi.fn() }));

const mockedCheck = vi.mocked(checkForUpdates);

describe("UpdateCheck", () => {
  beforeEach(() => mockedCheck.mockReset());

  it("reports a newer release and links to it", async () => {
    mockedCheck.mockResolvedValue({
      current_version: "0.1.0",
      latest_version: "0.2.0",
      update_available: true,
      release_url: "https://github.com/alevidals/peekr/releases/tag/v0.2.0",
    });
    const user = userEvent.setup();

    render(<UpdateCheck />);
    await user.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(await screen.findByText(/0.2.0/)).toBeInTheDocument();
    expect(screen.getByText(/You have/)).toHaveTextContent("0.1.0");
    expect(screen.getByRole("button", { name: "Release page" })).toBeInTheDocument();
  });

  it("reports that the running version is current", async () => {
    mockedCheck.mockResolvedValue({
      current_version: "0.1.0",
      latest_version: "0.1.0",
      update_available: false,
      release_url: "https://github.com/alevidals/peekr/releases",
    });
    const user = userEvent.setup();

    render(<UpdateCheck />);
    await user.click(screen.getByRole("button", { name: "Check for updates" }));

    expect(await screen.findByText(/You are on the latest version/)).toHaveTextContent("0.1.0");
  });
});