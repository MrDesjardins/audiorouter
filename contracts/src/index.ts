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
  parameters: Record<string, boolean | number | string>;
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

export interface RoutePath {
  nodes: EntityId[];
  edges: EntityId[];
  channelMaps: number[][];
}

export interface RouteInspection {
  destinationNode: EntityId;
  reachable: boolean;
  paths: RoutePath[];
}

export interface GraphPlanResult {
  planId: EntityId;
  baseRevision: number;
  expiresInMs: number;
  diff: unknown[];
  affectedDestinations: string[];
  warnings: string[];
  requiredScopes: string[];
}

export interface GraphCommitResult {
  sessionId: EntityId;
  revision: number;
  idempotentReplay?: boolean;
  activation?: Record<string, unknown>;
}

export type RecordingState = "armed" | "recording" | "paused" | "completed" | "failed";

export interface RecordingMetadata {
  title: string | null;
  artist: string | null;
  comment: string | null;
}

export interface RecordingRow extends RecordingMetadata {
  id: EntityId;
  sessionId: EntityId;
  recorderId: EntityId;
  path: string;
  format: "wav" | "flac";
  channels: 1 | 2;
  sampleRate: 44100 | 48000;
  frames: number;
  fileBytes: number;
  startTime: string;
  state: RecordingState;
  missing: boolean;
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

export interface StatusSnapshot {
  build: string;
  audio: "unavailable";
  deviceDiscovery: "available";
  reason: string;
  storage: "memory" | "sqlite";
  sessionCount: number;
  activeSessionCount: number;
  activeSessionIds: EntityId[];
  eventCursor: { backendEpoch: number; latestSequence: number };
}

export interface StateEvent {
  sequence: number;
  backendEpoch: number;
  resourceRevision: number;
  operationId: string | null;
  category: string;
  sessionId: EntityId | null;
}

export interface EventsSubscribeResult {
  backendEpoch: number;
  events: StateEvent[];
  nextSequence: number;
  resyncRequired?: boolean;
  snapshot?: { sessions: SessionListPage };
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
  | "clients.list"
  | "clients.authorize"
  | "clients.revoke"
  | "operations.get"
  | "operations.cancel"
  | "recordings.list"
  | "recordings.get"
  | "recordings.recovery"
  | "recordings.reveal"
  | "recordings.preview"
  | "recordings.setMetadata"
  | "recordings.rename"
  | "recordings.removeEntry"
  | "recordings.recycle"
  | "safety.setPrivacyMute"
  | "startup.get"
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
  | "sessions.start"
  | "session.stop"
  | "sessions.stop";

export type MethodParams = {
  "system.describe": undefined;
  "system.handshake": { protocolVersion: { major: number; minor: number } };
  "status.get": undefined;
  "system.diagnostics": undefined;
  "clients.list": undefined;
  "clients.authorize": { clientId: string; role: "observer" | "editor" | "operator" };
  "clients.revoke": { clientId: string };
  "operations.get": { operationId: string };
  "operations.cancel": { operationId: string };
  "recordings.list": { sessionId?: EntityId | null } | undefined;
  "recordings.get": { recordingId: EntityId };
  "recordings.recovery": { recordingId: EntityId };
  "recordings.reveal": { recordingId: EntityId };
  "recordings.preview": { recordingId: EntityId };
  "recordings.setMetadata": {
    recordingId: EntityId;
    title?: string | null;
    artist?: string | null;
    comment?: string | null;
  };
  "recordings.rename": { recordingId: EntityId; newPath: string };
  "recordings.removeEntry": { recordingId: EntityId };
  "recordings.recycle": { recordingId: EntityId; confirm?: boolean };
  "safety.setPrivacyMute": { muted: boolean };
  "startup.get": undefined;
  "devices.list": { cursor?: string; limit?: number } | undefined;
  "apps.list": undefined;
  "applications.list": undefined;
  "nodes.types": undefined;
  "routes.inspect": { sessionId: EntityId; destinationNode: EntityId };
  "graph.history": { sessionId: EntityId; cursor?: string; limit?: number };
  "graph.undoPlan": { sessionId: EntityId; baseRevision: number };
  "events.subscribe":
    | { afterSequence?: number; backendEpoch?: number; limit?: number; sessionId?: EntityId }
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
  "sessions.start": { sessionId: EntityId };
  "session.stop": { sessionId: EntityId };
  "sessions.stop": { sessionId: EntityId };
};

export type MethodResult = {
  "system.describe": DiscoveryDocument;
  "system.handshake": {
    compatible: true;
    requested: { major: number; minor: number };
    negotiated: { major: 1; minor: 0 };
    schemaVersion: number;
  };
  "status.get": StatusSnapshot;
  "system.diagnostics": Record<string, unknown>;
  "clients.list": Array<{ clientId: string; role: string; revoked: boolean }>;
  "clients.authorize": Record<string, unknown>;
  "clients.revoke": Record<string, unknown>;
  "operations.get": Record<string, unknown>;
  "operations.cancel": Record<string, unknown>;
  "recordings.list": RecordingRow[];
  "recordings.get": RecordingRow;
  "recordings.recovery": Record<string, unknown>;
  "recordings.reveal": Record<string, unknown>;
  "recordings.preview": Record<string, unknown>;
  "recordings.setMetadata": Record<string, unknown>;
  "recordings.rename": Record<string, unknown>;
  "recordings.removeEntry": Record<string, unknown>;
  "recordings.recycle": Record<string, unknown>;
  "safety.setPrivacyMute": Record<string, unknown>;
  "startup.get": Record<string, unknown>;
  "devices.list": unknown[];
  "apps.list": ApplicationInfo[];
  "applications.list": ApplicationInfo[];
  "nodes.types": DiscoveryDocument["nodeTypes"];
  "routes.inspect": RouteInspection;
  "graph.history": GraphHistoryPage;
  "graph.undoPlan": Record<string, unknown>;
  "events.subscribe": EventsSubscribeResult;
  "nodes.describe": DiscoveryDocument["nodeTypes"];
  "sessions.get": Session;
  "sessions.list": SessionListPage;
  "sessions.create": Record<string, unknown>;
  "sessions.duplicate": Record<string, unknown>;
  "sessions.delete": Record<string, unknown>;
  "graph.plan": GraphPlanResult;
  "graph.commit": GraphCommitResult;
  "session.start": Record<string, unknown>;
  "sessions.start": Record<string, unknown>;
  "session.stop": Record<string, unknown>;
  "sessions.stop": Record<string, unknown>;
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
