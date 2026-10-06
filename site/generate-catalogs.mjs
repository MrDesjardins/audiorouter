// Rebuild the static feature catalogs from the current backend method contract.
// Run from any directory: node site/generate-catalogs.mjs
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const domain = await readFile(resolve(root, "crates/domain/src/lib.rs"), "utf8");
const section = domain.slice(domain.indexOf("pub const API_METHODS:"), domain.indexOf("\n];", domain.indexOf("pub const API_METHODS:")));
const specs = [...section.matchAll(/ApiMethodSpec\s*\{([^}]*)\}/gs)].map(([, body]) => {
  const field = name => body.match(new RegExp(`${name}: (?:PermissionScope|SideEffectClass)::(\\w+)`))?.[1];
  const name = body.match(/name: "([^"]+)"/)?.[1];
  if (!name) return null;
  return { name, permission: field("permission"), effect: field("side_effect") };
}).filter(Boolean);
const desktopOnly = new Set(["recordings.setRoot", "devices.setAccess"]);
// Keep the public site focused on shipped, user-facing routes. Driver-track
// lifecycle APIs stay in local development Swagger until that work is public.
const developmentOnly = [/^virtualDevices\./, /^virtualRoutes\./, /^nativeBridges\./, /^nativeDuplex\./, /^nativeRenderSources\./];
const publicMethods = specs.filter(x => !desktopOnly.has(x.name) && !developmentOnly.some(re => re.test(x.name)));
const operations = publicMethods.map(x => ({
  method: "POST", path: `/api/v1/${x.name.replaceAll(".", "/")}`,
  operation: x.name, permission: x.permission, effect: x.effect
}));
for (const [method, path, operation] of [
  ["GET", "/api/v1/capabilities", "http.capabilities"],
  ["GET", "/api/v1/sessions", "http.sessions"],
  ["GET", "/api/v1/status", "http.status"],
  ["GET", "/api/v1/sessions/active", "http.sessions.active"],
  ["PUT", "/api/v1/sessions/active", "http.sessions.activate"]
]) operations.push({ method, path, operation, permission: method === "PUT" ? "SessionControl" : "Read", effect: "HTTP convenience route" });

const tools = [
  ["Physical input", "Sources", "Bring in a selected microphone or other Windows capture endpoint."],
  ["Application capture", "Sources", "Capture audio from a selected running application process tree."],
  ["Endpoint loopback", "Sources", "Capture the sound being rendered by a selected output endpoint."],
  ["Test signal", "Sources", "Generate a controlled tone to check a route and its levels."],
  ["Audio file", "Sources", "Play imported WAV or MP3 audio into a session, with optional looping."],
  ["Network receive", "Sources", "Receive an AudioRouter audio stream from another computer on your network."],
  ["Physical output", "Destinations", "Render a route to a selected Windows audio output."],
  ["Recorder", "Destinations", "Record one route branch to WAV, FLAC, or MP3."],
  ["Network send", "Destinations", "Send audio from a branch to another AudioRouter computer over UDP."],
  ["Mixer", "Routing", "Combine explicit upstream sources and set each input's level."],
  ["Input switch", "Routing", "Choose between input A and B with a crossfade."],
  ["Gain", "Level", "Set a precise gain in decibels."],
  ["Volume", "Level", "Set a simple level from 0 to 200 percent."],
  ["Mute", "Level", "Silence or restore a branch while retaining its route."],
  ["Meter", "Level", "Inspect peak, RMS, clipping, and reduction telemetry."],
  ["Parametric EQ", "Tone", "Shape up to sixteen enabled bands with shelf, pass, and notch filters."],
  ["Graphic EQ", "Tone", "Adjust ten familiar graphic equalizer bands."],
  ["Bass & Treble", "Tone", "Adjust low and high shelves with straightforward controls."],
  ["FIR Filter", "Tone", "Apply a supplied impulse response using convolution."],
  ["Compressor", "Dynamics", "Control peaks and even out the level with threshold, ratio, timing, and makeup."],
  ["Gate / Expander", "Dynamics", "Turn down audio below a threshold while preserving deliberate signal flow."],
  ["Limiter", "Dynamics", "Set a sample-peak ceiling with lookahead."],
  ["Duck", "Dynamics", "Lower a target source in response to another node's level or supported game phase."],
  ["Delay", "Time & pitch", "Add a controlled delay from 0 to 1,000 ms."],
  ["Time Shift", "Time & pitch", "Pause, jump back or forward, and return to live audio with a bounded buffer."],
  ["Pitch", "Time & pitch", "Shift pitch by semitones and cents in a streaming processor."],
  ["Dehum", "Cleanup", "Reduce mains hum and harmonics at 50 or 60 Hz."],
  ["Declick", "Cleanup", "Repair click and crackle transients."],
  ["Denoise", "Cleanup", "Learn a noise profile and reduce steady background noise."],
  ["Speech Denoise", "Cleanup", "Reduce noise with spectral processing tuned for speech."],
  ["Spectral Gate", "Cleanup", "Learn a per-frequency noise floor and turn down bands beneath it."],
  ["VST Plugin", "Plugins", "Host a compatible installed x64 VST3 effect or a qualified x64 VST2 effect in an isolated worker."],
].map(([name, category, description]) => {
  // Match the tool symbols used by Workbench.tsx; Duck uses its real SVG mark.
  const appGlyphs = {
    "Physical input": "◉", "Application capture": "application", "Endpoint loopback": "↶",
    "Test signal": "∿", "Audio file": "♫", "Network receive": "⇣",
    "Physical output": "◎", Recorder: "●", "Network send": "⇡", Mixer: "⋈",
    "Input switch": "⇄", Gain: "◢", Volume: "◖", Mute: "⊘", Meter: "▥",
    "Parametric EQ": "⌁", "Graphic EQ": "▤", "Bass & Treble": "♮",
    "FIR Filter": "⧉", Compressor: "⤓", "Gate / Expander": "⊐", Limiter: "⊤",
    Duck: "duck", Delay: "◷", "Time Shift": "↺", Pitch: "↟", Dehum: "≁",
    Declick: "⌇", Denoise: "░", "Speech Denoise": "☊", "Spectral Gate": "▥",
    "VST Plugin": "plugin"
  };
  return { name, category, description, icon: appGlyphs[name] ?? "◉" };
});

await writeFile(resolve(root, "site/assets/api-methods.json"), JSON.stringify({ backendMethods: publicMethods.length, operations }, null, 2) + "\n");
await writeFile(resolve(root, "site/assets/tool-catalog.json"), JSON.stringify({ nodeKinds: tools.length, tools }, null, 2) + "\n");
console.log(`Wrote ${publicMethods.length} public backend methods, ${operations.length} HTTP operations, and ${tools.length} available node kinds.`);
