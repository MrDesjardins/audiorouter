/** @vitest-environment jsdom */

import { describe, expect, it, vi } from "vitest";
import { afterEach } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { Session } from "@audiorouter/contracts";
import { appendDraftConnection, appendLibraryNode, needsNativePaths } from "./draft";
import { demoSession } from "./fixtures";
import { isNetworkAddress, NetworkNodeEditor, networkTelemetryText } from "./NetworkNodeEditor";

describe("network node addresses", () => {
  it("accepts IPv4/IPv6 literals and rejects names, blanks and malformed input", () => {
    expect(isNetworkAddress("192.168.1.20")).toBe(true);
    expect(isNetworkAddress("10.0.0.5")).toBe(true);
    expect(isNetworkAddress("fe80::1")).toBe(true);
    expect(isNetworkAddress("streaming-pc")).toBe(false);
    expect(isNetworkAddress("192.168.1")).toBe(false);
    expect(isNetworkAddress("256.1.1.1")).toBe(false);
    expect(isNetworkAddress(" 192.168.1.20")).toBe(false);
    expect(isNetworkAddress("")).toBe(false);
  });
});

describe("network node status", () => {
  it("summarises sending, waiting, receiving and problems in plain words", () => {
    expect(networkTelemetryText(null)).toBeNull();
    expect(networkTelemetryText({ direction: "send", sentPackets: 375, droppedPackets: 0, sendErrors: 0 })).toBe("Sending · 375 packets");
    expect(networkTelemetryText({ direction: "send", sentPackets: 10, droppedPackets: 2, sendErrors: 1 })).toBe("Sending · 10 packets · 2 dropped · 1 send errors");
    expect(networkTelemetryText({ direction: "receive", receivedPackets: 0 })).toBe("Waiting for audio from the sending computer");
    expect(networkTelemetryText({ direction: "receive", receivedPackets: 0, rejectedDatagrams: 4 })).toContain("4 packets from another address were ignored");
    expect(networkTelemetryText({ direction: "receive", receivedPackets: 900, bufferedMs: 38.6, lostPackets: 1, underruns: 1 })).toBe("Receiving · 900 packets · 39 ms buffered · 1 lost · 1 gaps");
  });
});

describe("network nodes in a draft", () => {
  it("creates stereo send/receive nodes with the default port and no address yet", () => {
    const withSend = appendLibraryNode(demoSession, "networkSend");
    const send = withSend.nodes.at(-1)!;
    expect(send.kind).toBe("networkSend");
    expect(send.parameters).toEqual({ port: 47800 });
    expect(send.ports).toEqual([{ name: "in", direction: "input", channels: 2 }]);
    const receive = appendLibraryNode(withSend, "networkReceive").nodes.at(-1)!;
    expect(receive.parameters).toEqual({ port: 47800, bufferMs: 40 });
    expect(receive.ports).toEqual([{ name: "out", direction: "output", channels: 2 }]);
  });

  it("always plays network routes on the multi-path worker", () => {
    const empty: Session = { ...demoSession, nodes: [], edges: [] };
    let session = appendLibraryNode(empty, "physicalInput");
    session = appendLibraryNode(session, "physicalOutput");
    const [input, output] = session.nodes;
    session = appendDraftConnection(session, input.id, "out", output.id, "in");
    expect(needsNativePaths(session)).toBe(false);
    expect(needsNativePaths(appendLibraryNode(session, "networkSend"))).toBe(true);
  });
});

describe("NetworkNodeEditor", () => {
  afterEach(cleanup);
  it("writes only a valid address and port to the draft", () => {
    const node = appendLibraryNode(demoSession, "networkSend").nodes.at(-1)!;
    const onChange = vi.fn();
    render(<NetworkNodeEditor node={node} disabled={false} telemetry={null} onChange={onChange} />);
    const address = screen.getByLabelText("Receiving computer's IP address");
    fireEvent.change(address, { target: { value: "gaming-pc" } });
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("alert").textContent).toContain("numeric IP address");
    fireEvent.change(address, { target: { value: "192.168.1.20" } });
    expect(onChange).toHaveBeenLastCalledWith("host", "192.168.1.20");
    fireEvent.change(screen.getByLabelText("Port"), { target: { value: "70000" } });
    expect(onChange).toHaveBeenCalledTimes(1);
    fireEvent.change(screen.getByLabelText("Port"), { target: { value: "48000" } });
    expect(onChange).toHaveBeenLastCalledWith("port", 48000);
    expect(screen.queryByLabelText("Buffer (ms)")).toBeNull();
  });

  it("offers the sender address and jitter buffer on a receive node", () => {
    const node = appendLibraryNode(demoSession, "networkReceive").nodes.at(-1)!;
    const onChange = vi.fn();
    render(<NetworkNodeEditor node={node} disabled={false} telemetry={{ direction: "receive", receivedPackets: 12, bufferedMs: 40 }} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("Sending computer's IP address"), { target: { value: "10.0.0.5" } });
    expect(onChange).toHaveBeenLastCalledWith("sender", "10.0.0.5");
    fireEvent.change(screen.getByLabelText("Buffer (ms)"), { target: { value: "5" } });
    expect(onChange).toHaveBeenCalledTimes(1);
    fireEvent.change(screen.getByLabelText("Buffer (ms)"), { target: { value: "80" } });
    expect(onChange).toHaveBeenLastCalledWith("bufferMs", 80);
    expect(screen.getByRole("status").textContent).toBe("Receiving · 12 packets · 40 ms buffered");
  });

  it("names this computer's real address while playing, and explains a closed port", () => {
    const receive = appendLibraryNode(demoSession, "networkReceive").nodes.at(-1)!;
    const view = render(<NetworkNodeEditor node={receive} disabled={false} telemetry={{ direction: "receive", receivedPackets: 0, thisAddress: "192.168.1.30" }} onChange={() => {}} />);
    const help = screen.getByText(/add a Network Send node/);
    expect(help.textContent).toContain("this computer's address 192.168.1.30 and port");
    expect(help.textContent).not.toContain("ipconfig");
    view.unmount();
    const send = appendLibraryNode(demoSession, "networkSend").nodes.at(-1)!;
    render(<NetworkNodeEditor node={send} disabled={false} telemetry={{ direction: "send", sentPackets: 40, sendErrors: 3, lastErrorCode: 10054, localAddress: "10.0.0.7" }} onChange={() => {}} />);
    expect(screen.getByText(/add a Network Receive node/).textContent).toContain("this computer's address 10.0.0.7 as the sender");
    expect(screen.getByRole("status").textContent).toContain("3 send errors (the receiving computer is not listening on this port)");
    // Stopped: no telemetry, the generic advice with ipconfig.
    expect(networkTelemetryText({ direction: "send", sentPackets: 1, sendErrors: 1, lastErrorCode: 10065 })).toBe("Sending · 1 packets · 1 send errors");
  });

  it("names the address audio really comes from and fixes the sender in one click", () => {
    const base = appendLibraryNode(demoSession, "networkReceive").nodes.at(-1)!;
    const node = { ...base, parameters: { ...base.parameters, sender: "192.168.1.20" } };
    const onChange = vi.fn();
    render(<NetworkNodeEditor node={node} disabled={false} telemetry={{ direction: "receive", receivedPackets: 0, rejectedDatagrams: 40, rejectedFrom: "192.168.1.51" }} onChange={onChange} />);
    expect(screen.getByRole("status").textContent).toContain("audio from 192.168.1.51 was ignored");
    expect(screen.getByRole("alert").textContent).toContain("arriving from 192.168.1.51");
    fireEvent.click(screen.getByRole("button", { name: "Use 192.168.1.51" }));
    expect(onChange).toHaveBeenLastCalledWith("sender", "192.168.1.51");
    expect((screen.getByLabelText("Sending computer's IP address") as HTMLInputElement).value).toBe("192.168.1.51");
  });
});
