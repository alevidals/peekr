import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "./ConfirmDialog";

describe("ConfirmDialog", () => {
  it("renders nothing while closed", () => {
    const { container } = render(
      <ConfirmDialog
        open={false}
        title="Remove account?"
        description="Saving will remove the account."
        confirmLabel="Remove account"
        cancelLabel="Keep account"
        onConfirm={() => {}}
        onCancel={() => {}}
      />
    );

    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("focuses the safe action and allows cancelling with Escape", async () => {
    const onCancel = vi.fn();
    const user = userEvent.setup();

    render(
      <ConfirmDialog
        open
        title="Remove account?"
        description="Saving will remove the account."
        confirmLabel="Remove account"
        cancelLabel="Keep account"
        onConfirm={() => {}}
        onCancel={onCancel}
      />
    );

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Keep account" })).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("confirms the destructive action", async () => {
    const onConfirm = vi.fn();
    const user = userEvent.setup();

    render(
      <ConfirmDialog
        open
        title="Remove account?"
        description="Saving will remove the account."
        confirmLabel="Remove account"
        cancelLabel="Keep account"
        onConfirm={onConfirm}
        onCancel={() => {}}
      />
    );

    await user.click(screen.getByRole("button", { name: "Remove account" }));
    expect(onConfirm).toHaveBeenCalledOnce();
  });
});
