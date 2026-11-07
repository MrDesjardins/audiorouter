// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import type { DiscoveryDocument, Session } from "@audiorouter/contracts";
import { RequestBuilder } from "./ApiRequestBuilder";
import { demoSession } from "./fixtures";

const nodeTypes = [
  {
    type: "gain@1",
    parameters: [{ name: "gainDb", type: "number", minimum: -60, maximum: 24, default: 0, unit: "dB" }],
  },
] as unknown as DiscoveryDocument["nodeTypes"];

/** What a backend refresh delivers: equal content in brand-new objects. */
const refreshed = (session: Session, gainDb = 0): Session => {
  const copy = structuredClone(session);
  copy.nodes.find((node) => node.id === "voice")!.parameters.gainDb = gainDb;
  return copy;
};

function setup(onSend: ComponentProps<typeof RequestBuilder>["onSend"] = vi.fn()) {
  const props = {
    activeSessionId: demoSession.id,
    nodeTypes,
    baseUrl: "http://127.0.0.1:17891",
    connected: true,
    onSend,
  };
  const view = render(<RequestBuilder {...props} sessions={[demoSession]} />);
  const show = (session: Session) => view.rerender(<RequestBuilder {...props} sessions={[session]} />);
  fireEvent.change(screen.getByLabelText("Request tool"), { target: { value: "voice" } });
  fireEvent.change(screen.getByLabelText("Request setting"), { target: { value: "gainDb" } });
  return { show, value: () => screen.getByLabelText<HTMLInputElement>("Request value") };
}

describe("RequestBuilder through a backend refresh", () => {
  afterEach(cleanup);

  it("keeps the Sent message when the refresh arrives after the send returns", async () => {
    const onSend = vi.fn().mockResolvedValue({ revision: 7 });
    const { show, value } = setup(onSend);
    fireEvent.change(value(), { target: { value: "3" } });
    await act(async () => void fireEvent.click(screen.getByRole("button", { name: "Send now" })));
    expect(screen.getByRole("status").textContent).toContain("Saved as revision 7");
    show(refreshed(demoSession, 3));
    expect(screen.getByRole("status").textContent).toContain("Saved as revision 7");
    expect(value().value).toBe("3");
  });

  it("does not overwrite a value being typed when the saved value did not change", () => {
    const { show, value } = setup();
    fireEvent.change(value(), { target: { value: "12" } });
    show(refreshed(demoSession));
    expect(value().value).toBe("12");
  });

  it("shows a new saved value that came from somewhere else", () => {
    const { show, value } = setup();
    show(refreshed(demoSession, 6));
    expect(value().value).toBe("6");
  });

  it("clears the message and loads the saved value when another setting is chosen", async () => {
    const { value } = setup(vi.fn().mockResolvedValue({ revision: 7 }));
    fireEvent.change(value(), { target: { value: "3" } });
    await act(async () => void fireEvent.click(screen.getByRole("button", { name: "Send now" })));
    fireEvent.change(screen.getByLabelText("Request tool"), { target: { value: "mic" } });
    expect(screen.queryByRole("status")).toBeNull();
  });
});
