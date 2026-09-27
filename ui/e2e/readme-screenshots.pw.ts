import path from "node:path";
import type { Session } from "../../contracts/src/index";
import { appendDraftConnection, appendLibraryNode, setNodeDraftName, setNodeDraftParameter, type LibraryNodeKind } from "../src/draft";
import { test, expect } from "./real-backend";

// Renders the README screenshots in docs/images from realistic sessions.
// Opt-in: set AUDIOROUTER_README_SHOTS=1 and run this file with Playwright.
const out = path.resolve("../docs/images");

class Builder {
  session: Session;
  constructor(id: string, name: string) { this.session = { id, name, schemaVersion: 1, revision: 0, nodes: [], edges: [] }; }
  add(kind: LibraryNodeKind, name: string, parameters: Record<string, number | string | boolean> = {}) {
    this.session = appendLibraryNode(this.session, kind);
    const id = this.session.nodes.at(-1)!.id;
    this.session = setNodeDraftName(this.session, id, name);
    for (const [key, value] of Object.entries(parameters)) this.session = setNodeDraftParameter(this.session, id, key, value);
    return id;
  }
  link(source: string, destination: string) {
    const src = this.session.nodes.find((node) => node.id === source)!;
    const dst = this.session.nodes.find((node) => node.id === destination)!;
    this.session = appendDraftConnection(this.session, source, src.ports.find((port) => port.direction === "output")!.name, destination, dst.ports.find((port) => port.direction === "input")!.name);
  }
  chain(...ids: string[]) { for (let index = 1; index < ids.length; index += 1) this.link(ids[index - 1], ids[index]); }
}

function streamerVoice() {
  const b = new Builder("streamer-voice", "Streamer voice chain");
  const mic = b.add("physicalInput", "Shure SM7B (USB interface)");
  const gate = b.add("gate", "Noise gate");
  const denoise = b.add("denoise", "Denoise");
  const eq = b.add("parametricEq", "Voice EQ", { band0Enabled: true, band0Type: "highPass", band0FrequencyHz: 90, band1Enabled: true, band1Type: "peaking", band1FrequencyHz: 250, band1GainDb: -3.5, band1Q: 1.2, band2Enabled: true, band2Type: "peaking", band2FrequencyHz: 3200, band2GainDb: 4, band2Q: 0.9, band3Enabled: true, band3Type: "highShelf", band3FrequencyHz: 9000, band3GainDb: 2.5, band3Q: 0.7 });
  const comp = b.add("compressor", "Compressor");
  const limiter = b.add("limiter", "Limiter");
  const discord = b.add("physicalOutput", "CABLE Input → Discord");
  const monitor = b.add("physicalOutput", "Headphones (monitor)");
  const take = b.add("recorder", "Podcast take");
  b.chain(mic, gate, denoise, eq, comp, limiter, discord);
  b.link(limiter, monitor);
  b.link(limiter, take);
  return b.session;
}

function gamingPc() {
  const b = new Builder("gaming-pc", "Gaming PC → streaming PC");
  const game = b.add("physicalInput", "Game audio (CABLE Output)");
  const gameVolume = b.add("volume", "Game level");
  const mic = b.add("physicalInput", "Microphone");
  const voice = b.add("speechDenoise", "Speech denoise");
  const voiceEq = b.add("parametricEq", "Voice EQ");
  const mixer = b.add("mixer", "Stream mix");
  const send = b.add("networkSend", "To streaming PC", { host: "192.168.1.50" });
  const headphones = b.add("physicalOutput", "Headphones");
  b.chain(game, gameVolume, mixer);
  b.chain(mic, voice, voiceEq, mixer);
  b.link(mixer, send);
  b.link(gameVolume, headphones);
  return b.session;
}

function streamingPc() {
  const b = new Builder("streaming-pc", "Streaming PC: receive and record");
  const receive = b.add("networkReceive", "From gaming PC");
  const level = b.add("gain", "Trim");
  const meter = b.add("meter", "Level meter");
  const obs = b.add("physicalOutput", "CABLE Input → OBS");
  const backup = b.add("recorder", "Backup recording");
  b.chain(receive, level, obs);
  b.link(level, meter);
  b.link(level, backup);
  return b.session;
}

test.skip(!process.env.AUDIOROUTER_README_SHOTS, "README screenshots are generated on request");
test.setTimeout(120_000);
test("README screenshots", async ({ page, backend }) => {
  for (const [index, session] of [streamerVoice(), gamingPc(), streamingPc()].entries()) {
    await backend.call("sessions.create", { session, idempotencyKey: `readme-${index}` });
  }
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const open = async (name: string) => {
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("combobox", { name: "Choose session" }).selectOption({ label: name });
    await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Tidy layout" }).click();
    await page.waitForTimeout(400);
  };
  const shot = (file: string) => page.screenshot({ path: path.join(out, file) });

  await open("Streamer voice chain");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await shot("streamer-voice-chain.png");
  await page.getByText("Voice EQ", { exact: true }).first().click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await page.locator(".main-content > .inspector").evaluate((element) => element.scrollTo(0, 0));
  await page.locator(".advanced-eq").scrollIntoViewIfNeeded();
  await shot("advanced-eq.png");

  await open("Gaming PC → streaming PC");
  await page.getByText("To streaming PC", { exact: true }).first().click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await shot("gaming-pc-network-send.png");

  await open("Streaming PC: receive and record");
  await page.getByLabel("Color theme").selectOption("light");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await shot("streaming-pc-light.png");
  await page.getByLabel("Color theme").selectOption("dark");
});
