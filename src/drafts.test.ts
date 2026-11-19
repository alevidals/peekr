import { describe, expect, it } from "vitest";
import {
  blankDraft,
  configToDrafts,
  draftsToAccounts,
  isDraftComplete,
  parseRepos,
} from "./drafts";
import type { Config } from "./types";

describe("drafts", () => {
  it("trims repository lines and drops empties", () => {
    expect(parseRepos(" web \n\n api\nweb ")).toEqual(["web", "api", "web"]);
  });

  it("requires the fields each provider needs", () => {
    expect(isDraftComplete(blankDraft("bitbucket"))).toBe(false);

    const bitbucket = {
      ...blankDraft("bitbucket"),
      workspace: "acme",
      token: "token",
    };
    expect(isDraftComplete(bitbucket)).toBe(false);
    expect(isDraftComplete({ ...bitbucket, reposText: "web" })).toBe(true);

    const github = {
      ...blankDraft("github"),
      workspace: "acme",
      token: "token",
    };
    expect(isDraftComplete(github)).toBe(true);
  });

  it("keeps a stored token when the draft does not type a new one", () => {
    const config: Config = {
      polling_interval_secs: 120,
      theme: "system",
      appearance: {
        palette: "indigo",
        accent: null,
        density: "comfortable",
        text_size: "medium",
      },
      accounts: [
        {
          id: "saved",
          kind: "github",
          workspace: "acme",
          repos: ["web"],
          has_token: true,
        },
      ],
    };

    const drafts = configToDrafts(config);
    expect(drafts[0]).toMatchObject({
      id: "saved",
      workspace: "acme",
      hasToken: true,
      reposText: "web",
    });
    expect(draftsToAccounts(drafts)).toEqual([
      { id: "saved", kind: "github", workspace: "acme", repos: ["web"], token: undefined },
    ]);
  });

  it("starts with one empty draft when there are no accounts", () => {
    const drafts = configToDrafts({
      polling_interval_secs: 120,
      theme: "system",
      appearance: {
        palette: "indigo",
        accent: null,
        density: "comfortable",
        text_size: "medium",
      },
      accounts: [],
    });

    expect(drafts).toHaveLength(1);
    expect(drafts[0].workspace).toBe("");
    expect(drafts[0].id).toBeUndefined();
  });
});
