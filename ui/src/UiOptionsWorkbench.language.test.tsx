/** @vitest-environment jsdom */
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import { UiOptionsWorkbench } from "./UiOptionsWorkbench";
import type { CustomThemeLibrary } from "./customTheme";

afterEach(cleanup);

const noThemes: CustomThemeLibrary = { selectedId: "", themes: [] };

function renderOptions(language: string | null) {
  const onLanguage = vi.fn();
  render(
    <UiOptionsWorkbench
      theme="dark"
      onTheme={() => undefined}
      language={language}
      onLanguage={onLanguage}
      customThemes={noThemes}
      onCustomThemes={() => undefined}
      flowAnimation="focus"
      onFlowAnimation={() => undefined}
      liveAutosave
      onLiveAutosave={() => undefined}
    />,
  );
  return onLanguage;
}

describe("Appearance language setting", () => {
  it("shows System default and the discovered language names", () => {
    renderOptions(null);
    const select = screen.getByLabelText("Language") as HTMLSelectElement;
    expect(select.value).toBe("system");
    expect([...select.options].map((option) => option.textContent)).toContain("English");
    expect([...select.options].map((option) => option.textContent)).toContain("System default");
  });

  it("returns the explicit tag or null for System default", () => {
    const first = renderOptions(null);
    fireEvent.change(screen.getByLabelText("Language"), { target: { value: "en" } });
    expect(first).toHaveBeenLastCalledWith("en");
    cleanup();

    const second = renderOptions("en");
    expect((screen.getByLabelText("Language") as HTMLSelectElement).value).toBe("en");
    fireEvent.change(screen.getByLabelText("Language"), { target: { value: "system" } });
    expect(second).toHaveBeenLastCalledWith(null);
  });
});
