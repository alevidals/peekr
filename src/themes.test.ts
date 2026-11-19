import { describe, expect, it } from "vitest";
import { NAMED_THEMES, THEME_OPTIONS } from "./themes";

describe("themes", () => {
  it("offers every theme exactly once", () => {
    const values = THEME_OPTIONS.map((option) => option.value);
    expect(new Set(values).size).toBe(values.length);
    expect(values).toContain("system");
    expect(values).toContain("catppuccinMocha");
  });

  it("maps named themes to their own light or dark scheme", () => {
    for (const theme of NAMED_THEMES) {
      const option = THEME_OPTIONS.find((item) => item.value === theme.id);
      expect(option, theme.id).toBeDefined();
      expect(option?.theme).toBe(theme.dark ? "dark" : "light");
      expect(option?.palette).toBe(theme.id);
    }
  });
});
