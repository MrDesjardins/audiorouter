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
  permission: PermissionScope;
  sideEffect: SideEffectClass;
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
  }>;
  limits: { maxNodesPerSession: number; maxEdgesPerSession: number };
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
  error: { code: number; message: string; data?: unknown };
}

export type JsonRpcResponse<Result = unknown> =
  | JsonRpcSuccess<Result>
  | JsonRpcError;

export type ImplementedMethod =
  | "system.describe"
  | "status.get"
  | "devices.list"
  | "apps.list"
  | "nodes.types"
  | "routes.inspect"
  | "graph.history"
  | "graph.undoPlan"
  | "events.subscribe"
  | "graph.plan"
  | "graph.commit"
  | "session.start"
  | "session.stop";
