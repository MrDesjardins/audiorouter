// Generated contract surface for the shared AudioRouter JSON-RPC API.
// Keep this file aligned with crates/domain and crates/protocol; UI/CLI/MCP
// adapters must use these shapes rather than private request paths.

export type EntityId = string;

export type NodeKind =
  | "physicalInput"
  | "applicationCapture"
  | "endpointLoopback"
  | "physicalOutput"
  | "mixer"
  | "gain"
  | "mute"
  | "meter";

export type PortDirection = "input" | "output";

export interface Port {
  name: string;
  direction: PortDirection;
  channels: 1 | 2;
}

export interface Node {
  id: EntityId;
  kind: NodeKind;
  name: string;
  enabled: boolean;
  bypass: boolean;
  ports: Port[];
}

export interface Edge {
  id: EntityId;
  sourceNode: EntityId;
  sourcePort: string;
  destinationNode: EntityId;
  destinationPort: string;
  matrix: number[];
  enabled: boolean;
}

export interface Session {
  id: EntityId;
  name: string;
  schemaVersion: number;
  revision: number;
  nodes: Node[];
  edges: Edge[];
}

export interface SessionListPage {
  items: Session[];
  nextCursor: EntityId | null;
}

export interface GraphHistoryPage {
  items: Session[];
  nextCursor: string | null;
}

export type PermissionScope =
  | "read"
  | "graphWrite"
  | "sessionControl"
  | "capture"
  | "record"
  | "deviceAdministration";

export type SideEffectClass =
  | "readOnly"
  | "planOnly"
  | "mutating"
  | "externalOperation";

export interface MethodDescription {
  name: string;
  description: string;
  permission: PermissionScope;
  sideEffect: SideEffectClass;
  inputSchema: unknown;
  outputSchema: unknown;
}

export interface DiscoveryDocument {
  protocolVersion: { major: number; minor: number };
  schemaVersion: number;
  build: string;
  methods: MethodDescription[];
  nodeTypes: Array<{
    type: `${NodeKind}@${number}`;
    availability: { status: "available" | "unavailable"; reason?: string };
    realtimeCostClass: string;
    parameters: Array<{
      name: string;
      type: string;
      unit?: string;
      minimum?: number;
      maximum?: number;
      default?: boolean | number | string;
    }>;
  }>;
  limits: {
    maxNodesPerSession: number;
    maxEdgesPerSession: number;
    maxNodesGlobal: number;
    maxEdgesGlobal: number;
    maxActiveSessions: number;
  };
  events: {
    stateCategories: string[];
    meterReplay: false;
    retention: { maxEvents: number; maxAgeSeconds: number };
  };
}

export interface ApplicationInfo {
  processId: number;
  executable: string;
  creationTime100ns: string | null;
}

export interface JsonRpcRequest<Params = unknown> {
  jsonrpc: "2.0";
  id?: string | number | null;
  method: string;
  params?: Params;
}

export interface JsonRpcSuccess<Result = unknown> {
  jsonrpc: "2.0";
  id: string | number | null;
  result: Result;
}

export interface JsonRpcError {
  jsonrpc: "2.0";
  id: string | number | null;
  error: {
    code: number;
    message: string;
    data?: ApplicationErrorData;
  };
}

export interface ApplicationErrorData {
  code: string;
  fieldPath: string | null;
  resourceIds: EntityId[];
  retryable: boolean;
  remediation: string;
  retryAfterMs?: number;
}

export type JsonRpcResponse<Result = unknown> =
  | JsonRpcSuccess<Result>
  | JsonRpcError;

export type ImplementedMethod =
  | "system.describe"
  | "system.handshake"
  | "status.get"
  | "system.diagnostics"
  | "devices.list"
  | "apps.list"
  | "applications.list"
  | "nodes.types"
  | "routes.inspect"
  | "graph.history"
  | "graph.undoPlan"
  | "events.subscribe"
  | "nodes.describe"
  | "sessions.get"
  | "sessions.list"
  | "sessions.create"
  | "sessions.duplicate"
  | "sessions.delete"
  | "graph.plan"
  | "graph.commit"
  | "session.start"
  | "session.stop";

export type MethodParams = {
  "system.describe": undefined;
  "system.handshake": { protocolVersion: { major: number; minor: number } };
  "status.get": undefined;
  "system.diagnostics": undefined;
  "devices.list": { cursor?: string; limit?: number } | undefined;
  "apps.list": undefined;
  "applications.list": undefined;
  "nodes.types": undefined;
  "routes.inspect": { sessionId: EntityId; destinationNode: EntityId };
  "graph.history": { sessionId: EntityId; cursor?: string; limit?: number };
  "graph.undoPlan": { sessionId: EntityId; baseRevision: number };
  "events.subscribe":
    | { afterSequence?: number; limit?: number; sessionId?: EntityId }
    | undefined;
  "nodes.describe": undefined;
  "sessions.get": { sessionId: EntityId };
  "sessions.list": { cursor?: string; limit?: number } | undefined;
  "sessions.create": { session: Session };
  "sessions.duplicate": {
    sourceSessionId: EntityId;
    sessionId: EntityId;
    name?: string;
  };
  "sessions.delete": { sessionId: EntityId };
  "graph.plan": { sessionId: EntityId; baseRevision: number; candidate: Session };
  "graph.commit": { planId: EntityId; baseRevision: number; idempotencyKey: string };
  "session.start": { sessionId: EntityId };
  "session.stop": { sessionId: EntityId };
};

export type MethodResult = {
  "system.describe": DiscoveryDocument;
  "system.handshake": {
    compatible: true;
    requested: { major: number; minor: number };
    negotiated: { major: 1; minor: 0 };
    schemaVersion: number;
  };
  "status.get": Record<string, unknown>;
  "system.diagnostics": Record<string, unknown>;
  "devices.list": unknown[];
  "apps.list": ApplicationInfo[];
  "applications.list": ApplicationInfo[];
  "nodes.types": DiscoveryDocument["nodeTypes"];
  "routes.inspect": Record<string, unknown>;
  "graph.history": GraphHistoryPage;
  "graph.undoPlan": Record<string, unknown>;
  "events.subscribe": Record<string, unknown>;
  "nodes.describe": DiscoveryDocument["nodeTypes"];
  "sessions.get": Session;
  "sessions.list": SessionListPage;
  "sessions.create": Record<string, unknown>;
  "sessions.duplicate": Record<string, unknown>;
  "sessions.delete": Record<string, unknown>;
  "graph.plan": Record<string, unknown>;
  "graph.commit": Record<string, unknown>;
  "session.start": Record<string, unknown>;
  "session.stop": Record<string, unknown>;
};

export interface RpcTransport {
  send(request: JsonRpcRequest): Promise<JsonRpcResponse>;
}

export class AudioRouterRpcError extends Error {
  readonly code: number;
  readonly data?: ApplicationErrorData;

  constructor(error: JsonRpcError["error"]) {
    super(error.message);
    this.name = "AudioRouterRpcError";
    this.code = error.code;
    this.data = error.data;
  }
}

export interface AudioRouterClient {
  request<M extends ImplementedMethod>(
    method: M,
    params: MethodParams[M],
  ): Promise<MethodResult[M]>;
}

/** Create the shared typed client over any framed/local transport adapter. */
export function createAudioRouterClient(transport: RpcTransport): AudioRouterClient {
  let nextId = 1;
  return {
    async request(method, params) {
      const response = await transport.send({
        jsonrpc: "2.0",
        id: nextId++,
        method,
        ...(params === undefined ? {} : { params }),
      });
      if ("error" in response) {
        throw new AudioRouterRpcError(response.error);
      }
      return response.result as MethodResult[typeof method];
    },
  };
}
