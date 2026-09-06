import type { Session } from "@audiorouter/contracts";
import { appendDraftConnection } from "./draft";
import { demoSession } from "./fixtures";

export type TemplateId = "gaming-discord" | "processed-microphone" | "mix-minus";

function clone(session: Session, name: string): Session {
  return {
    ...session,
    name,
    nodes: session.nodes.map((node) => ({ ...node, parameters: { ...node.parameters }, ports: node.ports.map((port) => ({ ...port })) })),
    edges: session.edges.map((edge) => ({ ...edge, matrix: [...edge.matrix] })),
  };
}

function voiceTemplate(name: string): Session {
  let session = clone(demoSession, name);
  session = { ...session, edges: [] };
  session = appendDraftConnection(session, "mic", "out", "voice", "in");
  return appendDraftConnection(session, "voice", "out", "headphones", "in");
}

export function templateSession(id: TemplateId): Session {
  switch (id) {
    case "gaming-discord":
      return voiceTemplate("Gaming + Discord");
    case "processed-microphone": {
      const session = voiceTemplate("Processed microphone");
      return { ...session, nodes: session.nodes.map((node) => node.id === "voice" ? { ...node, name: "Processed voice gain", parameters: { gainDb: -3 } } : node) };
    }
    case "mix-minus": {
      const session = voiceTemplate("Mix-minus conversation");
      return { ...session, nodes: session.nodes.map((node) => node.id === "voice" ? { ...node, name: "Call input (mic only)" } : node) };
    }
  }
}
