//! Optional loopback adapter. No state or audio execution is owned here.
use crate::http_accept::{adapter_error_body, close_gracefully, reply_with, ConnectionPool};
use audiorouter_protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX_BODY: usize = 4 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
/// Forward one request to the backend with its correlation ID (P2-3).
pub type Forward = dyn Fn(&JsonRpcRequest, &str) -> Result<JsonRpcResponse, String> + Send + Sync;

pub struct HttpApi {
    pub port: u16,
    pub lan: Option<Ipv4Addr>,
    pub token: String,
    /// Dropping the pool stops the listeners and workers.
    _pool: ConnectionPool,
}
impl HttpApi {
    #[cfg(test)]
    pub fn start(port: u16, forward: Arc<Forward>) -> Result<Self, String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| "Cannot securely generate API token")?;
        Self::start_with_token(
            port,
            bytes.iter().map(|b| format!("{b:02x}")).collect(),
            forward,
        )
    }
    #[cfg(test)]
    pub fn start_with_token(
        port: u16,
        token: String,
        forward: Arc<Forward>,
    ) -> Result<Self, String> {
        Self::start_on(port, token, None, forward)
    }
    /// Loopback is always served. `lan` adds one listener on that exact
    /// private or link-local address of this PC (HTTP-09); peers on it must
    /// themselves be private or link-local. Never binds every interface.
    pub fn start_on(
        port: u16,
        token: String,
        lan: Option<Ipv4Addr>,
        forward: Arc<Forward>,
    ) -> Result<Self, String> {
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid API token".into());
        }
        if let Some(address) = lan {
            if !lan_address_allowed(address) {
                return Err(format!("{address} is not a private local-network address. Choose this PC's home or office network address."));
            }
        }
        // Listeners stay blocking: each accept thread waits in the kernel
        // until a connection arrives, with no idle wake-ups or polling delay.
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .map_err(|_| format!("Cannot open localhost port {port}. Choose another port or stop the application using it."))?;
        let port = listener
            .local_addr()
            .map_err(|_| "Cannot read API port")?
            .port();
        let mut listeners = vec![(listener, false)];
        if let Some(address) = lan {
            let network = TcpListener::bind((address, port))
                .map_err(|_| format!("Cannot open port {port} on {address}. Check that this PC still has that address, or choose another port."))?;
            listeners.push((network, true));
        }
        let mut hosts = vec![format!("127.0.0.1:{port}")];
        hosts.extend(lan.map(|address| format!("{address}:{port}")));
        let describe = forward(
            &rpc("system.describe", None),
            &audiorouter_protocol::diagnostics::new_correlation_id(),
        )?
        .result
        .ok_or("Backend discovery unavailable; reconnect before starting API")?;
        let schema = openapi(&describe, port, lan);
        let rate = Mutex::new((Instant::now(), 40f64));
        let secret = token.clone();
        let pool = ConnectionPool::start(
            listeners,
            peer_allowed,
            Arc::new(move |stream: &mut TcpStream, stop: &AtomicBool| {
                let _ = handle(stream, &hosts, &secret, &schema, &forward, stop, &rate);
            }),
        )?;
        Ok(Self {
            port,
            lan,
            token,
            _pool: pool,
        })
    }
}

fn rpc(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".into(),
        id: Some(json!("localhost-http")),
        method: method.into(),
        params,
    }
}
/// Addresses a local-network listener may use: RFC 1918 private ranges and
/// IPv4 link-local (a direct cable). Public, CGNAT and VPN-overlay addresses
/// are refused so a listener cannot face the internet by mistake.
pub fn lan_address_allowed(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local()
}
/// The loopback listener serves only this PC; the network listener serves
/// this PC and private/link-local peers.
fn peer_allowed(peer: IpAddr, network: bool) -> bool {
    let peer = match peer {
        IpAddr::V4(peer) => peer,
        IpAddr::V6(peer) if peer.is_loopback() => return true,
        IpAddr::V6(peer) => match peer.to_ipv4_mapped() {
            Some(peer) => peer,
            None => return false,
        },
    };
    peer.is_loopback() || (network && lan_address_allowed(peer))
}
/// Exact Host check: one of the listener addresses with the active port.
/// A browser Origin, when sent, must be the same Host.
fn host_allowed(host: Option<&str>, origin: Option<&str>, hosts: &[String]) -> bool {
    let Some(host) = host.filter(|host| hosts.iter().any(|allowed| allowed == host)) else {
        return false;
    };
    origin.is_none_or(|origin| origin == format!("http://{host}"))
}
/// Methods only the desktop window may call: approving a recording folder is
/// the user's own file-root decision (REC-07, SEC authorization UX), so a
/// token holder on localhost cannot redirect recordings.
/// Methods the HTTP API refuses: they need the person at the AudioRouter
/// window (a folder choice, device consent, OBS changes after a preview the
/// person confirmed, DEC-22). Reading OBS (`obs.status`, `obs.refresh`) stays
/// available.
#[cfg(test)]
const DESKTOP_ONLY_METHODS: &[&str] = &[
    "sessions.approveExport",
    "sessions.exportFile",
    "recordings.setRoot",
    "devices.setAccess",
    "obs.connect",
    "obs.disconnect",
    "obs.preview",
    "obs.apply",
    "obs.restore",
    "clients.authorize",
    "clients.revoke",
];

enum HttpAccess {
    Open,
    LocalPaths(&'static [&'static str]),
    DesktopOnly(&'static str),
}

fn http_access(method: &str) -> HttpAccess {
    match method {
        "sessions.approveExport" => {
            HttpAccess::DesktopOnly("Approve an export file in the AudioRouter window")
        }
        "sessions.exportFile" => {
            HttpAccess::DesktopOnly("Export a session in the AudioRouter window")
        }
        "recordings.setRoot" => {
            HttpAccess::DesktopOnly("Choose the recording folder in the AudioRouter window")
        }
        "devices.setAccess" => {
            HttpAccess::DesktopOnly("Allow audio device access in the AudioRouter window")
        }
        "clients.authorize" => {
            HttpAccess::DesktopOnly("Enroll an assistant or app in the AudioRouter window")
        }
        "clients.revoke" => {
            HttpAccess::DesktopOnly("Remove an assistant or app in the AudioRouter window")
        }
        "obs.connect" => HttpAccess::DesktopOnly("Connect to OBS in the AudioRouter window"),
        "obs.disconnect" => {
            HttpAccess::DesktopOnly("Disconnect from OBS in the AudioRouter window")
        }
        "obs.preview" => HttpAccess::DesktopOnly("Preview OBS changes in the AudioRouter window"),
        "obs.apply" => HttpAccess::DesktopOnly("Apply OBS changes in the AudioRouter window"),
        "obs.restore" => HttpAccess::DesktopOnly("Restore OBS settings in the AudioRouter window"),
        "plugins.scan" => HttpAccess::LocalPaths(&["directory", "shellPath"]),
        "plugins.list" | "plugins.retry" => HttpAccess::LocalPaths(&["directory"]),
        "plugins.inspect" | "plugins.parameters" | "plugins.buses" | "sessions.importFile" => {
            HttpAccess::LocalPaths(&["path"])
        }
        "plugins.applyPreset" => HttpAccess::LocalPaths(&["presetPath"]),
        "nativeBridges.prepare" => {
            HttpAccess::LocalPaths(&["renderMappingPath", "captureMappingPath"])
        }
        _ => HttpAccess::Open,
    }
}

/// Check the spelling before touching a caller-selected filesystem path.
/// HTTP requests are forwarded with the window's grant, which may name a
/// network folder, so this adapter keeps its own refusal; the backend
/// applies `is_network_path` to every other client. A chosen file or folder
/// parameter refuses any `\\` spelling, even a local device path.
fn network_path(text: &str) -> bool {
    text.starts_with(r"\\") || text.starts_with("//") || audiorouter_control::is_network_path(text)
}

pub fn operation_path(name: &str) -> String {
    format!("/api/v1/{}", name.replace('.', "/"))
}
pub fn openapi(describe: &Value, port: u16, lan: Option<Ipv4Addr>) -> Value {
    let mut paths = serde_json::Map::new();
    for method in describe["methods"].as_array().into_iter().flatten() {
        let Some(name) = method["name"].as_str() else {
            continue;
        };
        if matches!(http_access(name), HttpAccess::DesktopOnly(_)) {
            continue;
        }
        paths.insert(operation_path(name), json!({"post": {
            "operationId": name, "tags": [name.split('.').next().unwrap_or("API")],
            "summary": method["description"],
            "description": format!("Permission: {}. Side effect: {}. Backend validation remains authoritative.", method["permission"], method["sideEffect"]),
            "requestBody": {"required": true, "content": {"application/json": {"schema": method["inputSchema"]}}},
            "responses": error_responses(json!({"200": {"description": "Backend result; inspect activation for durable graph commits", "content": {"application/json": {"schema": method["outputSchema"]}}}})),
            "security": [{"bearerAuth": []}]
        }}));
    }
    for (path, name) in [
        ("capabilities", "system.describe"),
        ("sessions", "sessions.list"),
        ("status", "status.get"),
        ("virtualCable/status", "virtualCable.status"),
    ] {
        let schema = describe["methods"]
            .as_array()
            .and_then(|methods| methods.iter().find(|method| method["name"] == name))
            .map(|method| method["outputSchema"].clone())
            .unwrap_or(json!({}));
        // Merge: a GET alias may share its path with the method's POST
        // operation (virtualCable/status), which must stay documented.
        let entry = paths
            .entry(format!("/api/v1/{path}"))
            .or_insert_with(|| json!({}));
        let summary = describe["methods"]
            .as_array()
            .and_then(|methods| methods.iter().find(|method| method["name"] == name))
            .map(|method| method["description"].clone())
            .unwrap_or(Value::Null);
        if let Some(operations) = entry.as_object_mut() {
            let mut get = json!({"operationId": format!("http.{path}"), "tags": [name.split('.').next().unwrap_or("API")], "summary": summary, "description": format!("Same as POST /api/v1/{} with no body.", name.replace('.', "/")), "security": [{"bearerAuth": []}], "responses": error_responses(json!({"200": {"description": "Backend result", "content": {"application/json": {"schema": schema}}}}))});
            if path == "sessions" {
                get["description"] = json!("Same as POST /api/v1/sessions/list. Page with the cursor and limit query parameters; pass the returned nextCursor to read the next page.");
                get["parameters"] = json!([
                    {"name": "cursor", "in": "query", "required": false, "schema": {"type": "string"}, "description": "nextCursor from the previous page"},
                    {"name": "limit", "in": "query", "required": false, "schema": {"type": "integer", "minimum": 1, "maximum": 500}}
                ]);
            }
            operations.insert("get".into(), get);
        }
    }
    let active_get = describe["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|method| method["name"] == "sessions.active.get")
        .map(|method| method["outputSchema"].clone())
        .unwrap_or(json!({}));
    let active_set = describe["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|method| method["name"] == "sessions.active.set");
    let active_path = paths
        .entry("/api/v1/sessions/active")
        .or_insert_with(|| json!({}));
    active_path["get"] = json!({ "operationId": "http.sessions.active", "tags": ["sessions"], "summary": "Read the session open in AudioRouter", "responses": error_responses(json!({ "200": { "description": "Currently selected editing session; audio is not started", "content": { "application/json": { "schema": active_get } } } })), "security": [{"bearerAuth": []}] });
    active_path["put"] = json!({ "operationId": "http.sessions.activate", "tags": ["sessions"], "summary": "Select the editing session without starting audio", "requestBody": { "required": true, "content": { "application/json": { "schema": active_set.map(|method| method["inputSchema"].clone()).unwrap_or(json!({})) } } }, "responses": error_responses(json!({ "200": { "description": "Selected session", "content": { "application/json": { "schema": active_set.map(|method| method["outputSchema"].clone()).unwrap_or(json!({})) } } } })), "security": [{"bearerAuth": []}] });
    json!({"openapi": "3.1.0", "info": {"title": "AudioRouter local API", "version": env!("CARGO_PKG_VERSION"), "description": "Errors return {\"error\": {\"code\", \"message\", \"data\"}}. Branch on error.data.code (stable), never on the message text."}, "servers": std::iter::once(format!("http://127.0.0.1:{port}")).chain(lan.map(|address| format!("http://{address}:{port}"))).map(|url| json!({"url": url})).collect::<Vec<_>>(), "paths": paths, "components": {"securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}}, "schemas": {"Error": error_schema()}, "headers": {"RetryAfter": {"description": "Seconds to wait before retrying", "schema": {"type": "integer", "minimum": 1}}}}})
}

/// The error body every non-2xx response carries.
fn error_schema() -> Value {
    json!({
        "type": "object",
        "required": ["error"],
        "properties": { "error": {
            "type": "object",
            "required": ["message"],
            "properties": {
                "code": { "type": "integer", "description": "JSON-RPC error code, when the backend answered." },
                "message": { "type": "string", "description": "Plain-language reason; do not parse it." },
                "data": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "Stable machine code, for example permissionDenied, notFound, revisionConflict, idempotencyConflict, rateLimited." },
                        "fieldPath": { "type": ["string", "null"] },
                        "resourceIds": { "type": "array", "items": { "type": "string" } },
                        "retryable": { "type": "boolean" },
                        "remediation": { "type": "string" },
                        "retryAfterMs": { "type": "integer" }
                    }
                }
            }
        } }
    })
}

/// Add the documented error responses (each with the Error body) to an
/// operation's success response. Mirrors `http_status_for_error_code` and
/// the adapter's own refusals.
fn error_responses(mut responses: Value) -> Value {
    let error = |description: &str| json!({ "description": description, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Error" } } } });
    let entries = [
        ("400", error("Invalid request: malformed JSON, a parameter the schema refuses, or an unsupported query string")),
        ("401", error("Missing or wrong bearer token")),
        ("403", error("Permission denied, Windows refused the device, a window-only action, or a network location")),
        ("404", error("Unknown operation, or the named session, plan or resource does not exist")),
        ("409", error("The current state refuses the request: a newer revision, a reused idempotency key, an expired or too many plans, a busy device, a node not set up, or nothing to undo")),
        ("413", error("JSON body larger than 4 MiB")),
        ("500", error("Internal backend error")),
        ("503", error("Backend unavailable, its database cannot be used, or Windows audio is restarting; retry later")),
    ];
    for (status, body) in entries {
        responses[status] = body;
    }
    let mut limited = error("Rate limited; wait for Retry-After");
    limited["headers"] = json!({ "Retry-After": { "$ref": "#/components/headers/RetryAfter" } });
    responses["429"] = limited;
    responses
}

fn reply(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    reply_with_id(stream, status, content_type, body, None)
}

/// `request_id` is echoed as `X-Request-Id`; it is always a validated
/// correlation ID (`[A-Za-z0-9-]`, at most 32 characters), so it cannot
/// split or inject header lines.
fn reply_with_id(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    request_id: Option<&str>,
) -> std::io::Result<()> {
    let request_id = audiorouter_protocol::diagnostics::valid_correlation_id(request_id)
        .map(|id| format!("X-Request-Id: {id}\r\n"))
        .unwrap_or_default();
    reply_with(stream, status, content_type, &request_id, body)
}
fn fail(stream: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    reply(
        stream,
        status,
        "application/json",
        &adapter_error_body(status, message),
    )?;
    close_gracefully(
        stream,
        Duration::from_millis(20),
        Duration::from_millis(50),
        MAX_BODY,
    );
    Ok(())
}
/// The length of the request head including its blank line, once it has
/// arrived.
fn header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
}
fn token_matches(actual: &str, token: &str) -> bool {
    let expected = format!("Bearer {token}");
    actual.len() == expected.len()
        && actual
            .as_bytes()
            .iter()
            .zip(expected.as_bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}
fn handle(
    stream: &mut TcpStream,
    hosts: &[String],
    token: &str,
    schema: &Value,
    forward: &Arc<Forward>,
    stop: &AtomicBool,
    rate: &Mutex<(Instant, f64)>,
) -> std::io::Result<()> {
    // Listeners are blocking, but keep accepted sockets explicitly blocking:
    // clients send headers/body in separate packets, and bounded blocking
    // reads must wait for them instead of treating WouldBlock as disconnect.
    stream.set_nonblocking(false)?;
    // Silent sockets should not occupy all workers for the whole request
    // deadline. Once a client begins sending, retain the normal 2 s budget.
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let started = Instant::now();
    // Read in chunks (it was one system call per byte). Whatever arrives
    // after the blank line is the start of the body.
    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        if let Some(end) = header_end(&bytes) {
            break end;
        }
        if bytes.len() >= MAX_HEADERS || started.elapsed() > Duration::from_secs(2) {
            return fail(stream, 400, "Request headers too large or too slow");
        }
        let room = (MAX_HEADERS - bytes.len()).min(chunk.len());
        let count = stream.read(&mut chunk[..room])?;
        if count == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        if bytes.is_empty() {
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        }
        bytes.extend_from_slice(&chunk[..count]);
    };
    if header_end > MAX_HEADERS {
        return fail(stream, 400, "Request headers too large or too slow");
    }
    let buffered_body = bytes.split_off(header_end);
    let Ok(headers) = std::str::from_utf8(&bytes) else {
        return fail(stream, 400, "Invalid HTTP headers");
    };
    let mut lines = headers.split("\r\n");
    let first = lines
        .next()
        .unwrap_or_default()
        .split(' ')
        .collect::<Vec<_>>();
    if first.len() != 3 || first[2] != "HTTP/1.1" {
        return fail(stream, 400, "HTTP/1.1 required");
    }
    let (verb, path) = (first[0], first[1]);
    let mut fields = std::collections::HashMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            return fail(stream, 400, "Invalid header");
        };
        if key.trim() != key
            || fields
                .insert(key.to_ascii_lowercase(), value.trim())
                .is_some()
        {
            return fail(stream, 400, "Duplicate or invalid header");
        }
    }
    if !host_allowed(
        fields.get("host").copied(),
        fields.get("origin").copied(),
        hosts,
    ) {
        return fail(
            stream,
            403,
            "Only this API's exact Host and same-origin requests are allowed",
        );
    }
    if fields.contains_key("transfer-encoding") {
        return fail(
            stream,
            400,
            "Chunked bodies are unsupported; send a Content-Length",
        );
    }
    // Only the paged session list takes a query string (GET has no body),
    // and only its cursor and limit; everything else uses JSON parameters.
    let (path, query) = match path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (path, None),
    };
    let query_params = match query {
        None => None,
        Some(query) if verb == "GET" && path == "/api/v1/sessions" => {
            match paging_query(query) {
                Ok(params) => Some(params),
                Err(message) => return fail(stream, 400, &message),
            }
        }
        Some(_) => {
            return fail(
                stream,
                400,
                "Query strings are accepted only on GET /api/v1/sessions (cursor, limit); use JSON parameters",
            )
        }
    };
    if verb == "GET" {
        match path {
            "/docs" | "/docs/" => return reply(stream, 200, "text/html; charset=utf-8", b"<!doctype html><html><head><meta charset='utf-8'><title>AudioRouter API</title><link rel='stylesheet' href='/swagger-ui.css'></head><body><p><a href='/licenses/swagger-ui.txt'>Bundled Swagger UI licenses</a></p><div id='swagger-ui'></div><script src='/swagger-ui-bundle.js'></script><script src='/swagger-init.js'></script></body></html>"),
            "/swagger-ui.css" => return reply(stream, 200, "text/css", include_bytes!("../http-assets/swagger-ui.css")),
            "/swagger-ui-bundle.js" => return reply(stream, 200, "text/javascript", include_bytes!("../http-assets/swagger-ui-bundle.js")),
            "/swagger-init.js" => return reply(stream, 200, "text/javascript", b"SwaggerUIBundle({url:'/openapi.json',dom_id:'#swagger-ui',validatorUrl:null,persistAuthorization:false,queryConfigEnabled:false});"),
            "/openapi.json" => return reply(stream, 200, "application/json", schema.to_string().as_bytes()),
            "/licenses/swagger-ui.txt" => return reply(stream, 200, "text/plain; charset=utf-8", concat!(include_str!("../http-assets/LICENSE"), "\n", include_str!("../http-assets/NOTICE"), "\n", include_str!("../http-assets/swagger-ui-bundle.js.LICENSE.txt")).as_bytes()),
            _ => {}
        }
    }
    if !fields
        .get("authorization")
        .is_some_and(|actual| token_matches(actual, token))
    {
        return fail(
            stream,
            401,
            "Enter the bearer token from AudioRouter's API tab",
        );
    }
    let method = match (verb, path) {
        ("GET", "/api/v1/capabilities") => "system.describe".into(),
        ("GET", "/api/v1/sessions") => "sessions.list".into(),
        ("GET", "/api/v1/sessions/active") => "sessions.active.get".into(),
        ("PUT", "/api/v1/sessions/active") => "sessions.active.set".into(),
        ("GET", "/api/v1/status") => "status.get".into(),
        ("GET", "/api/v1/virtualCable/status") => "virtualCable.status".into(),
        ("POST", path) => {
            let Some(method) = path
                .strip_prefix("/api/v1/")
                .map(|name| name.replace('/', "."))
            else {
                return fail(stream, 404, "Unknown API resource");
            };
            if !audiorouter_domain::API_METHODS
                .iter()
                .any(|spec| spec.name == method)
            {
                return fail(stream, 404, "Unknown API operation");
            }
            if let HttpAccess::DesktopOnly(message) = http_access(&method) {
                return fail(stream, 403, message);
            }
            method
        }
        _ => return fail(stream, 405, "Use the documented HTTP method/resource"),
    };
    let length = match fields.get("content-length") {
        None => 0,
        Some(value) => match value.parse::<usize>() {
            Ok(length) => length,
            Err(_) => return fail(stream, 400, "Invalid Content-Length"),
        },
    };
    if length > MAX_BODY {
        return fail(stream, 413, "JSON body exceeds 4 MiB");
    }
    let params = if verb == "POST" || verb == "PUT" {
        if fields
            .get("content-type")
            .is_none_or(|value| value.split(';').next() != Some("application/json"))
        {
            return fail(stream, 400, "Content-Type must be application/json");
        }
        if let Some(expect) = fields.get("expect") {
            if !expect.eq_ignore_ascii_case("100-continue") {
                return fail(stream, 400, "Unsupported Expect header");
            }
            // Authenticate and bound Content-Length before allowing the body.
            stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
            stream.flush()?;
        }
        let mut body = vec![0u8; length];
        let mut offset = buffered_body.len().min(length);
        body[..offset].copy_from_slice(&buffered_body[..offset]);
        while offset < length {
            if started.elapsed() > Duration::from_secs(2) {
                return fail(stream, 400, "Request body deadline exceeded");
            }
            let count = stream.read(&mut body[offset..])?;
            if count == 0 {
                return fail(stream, 400, "Incomplete body");
            }
            offset += count;
        }
        match serde_json::from_slice::<Value>(&body) {
            Ok(value) if value.is_object() => Some(value),
            _ => return fail(stream, 400, "Expected a bounded JSON parameter object"),
        }
    } else {
        if length != 0 {
            return fail(stream, 400, "GET cannot have a body");
        }
        query_params
    };
    if let HttpAccess::LocalPaths(parameters) = http_access(&method) {
        if parameters.iter().any(|name| {
            params
                .as_ref()
                .and_then(|params| params.get(*name))
                .and_then(Value::as_str)
                .is_some_and(network_path)
        }) {
            return fail(stream, 403, "Over the API, choose a folder on this PC. Network folders can be chosen in the AudioRouter window.");
        }
    }
    // Any other location parameter, at any depth (a node's file inside a
    // graph candidate or an imported session), gets the backend's rule.
    if let Some(pointer) = params
        .as_ref()
        .and_then(audiorouter_control::network_location_parameter)
    {
        return fail(stream, 403, &format!("Over the API, choose a file or folder on this PC ({pointer} names a network location). Network folders can be chosen in the AudioRouter window."));
    }
    if stop.load(Ordering::Acquire) {
        return fail(stream, 503, "API stopped");
    }
    // One correlation ID per HTTP request: the caller's `X-Request-Id` when
    // it is a valid ID, otherwise a fresh one. It is forwarded to the backend,
    // logged by the shell and backend, and returned in the response.
    let request_id = request_correlation_id(fields.get("x-request-id").copied());
    {
        let mut budget = rate.lock().unwrap_or_else(|error| error.into_inner());
        budget.1 = (budget.1 + budget.0.elapsed().as_secs_f64() * 20.0).min(40.0);
        budget.0 = Instant::now();
        let cost = if audiorouter_domain::API_METHODS.iter().any(|spec| {
            spec.name == method && spec.side_effect == audiorouter_domain::SideEffectClass::ReadOnly
        }) {
            0.1
        } else {
            1.0
        };
        if budget.1 < cost {
            // The budget refills at 20 requests a second.
            drop(budget);
            reply_with(
                stream,
                429,
                "application/json",
                "Retry-After: 1\r\n",
                &adapter_error_body(429, "Local API rate exceeded; retry after one second"),
            )?;
            close_gracefully(
                stream,
                Duration::from_millis(20),
                Duration::from_millis(50),
                MAX_BODY,
            );
            return Ok(());
        }
        budget.1 -= cost;
    }
    match forward(&rpc(&method, params), &request_id) {
        Ok(response) => match response.error {
            Some(error) => {
                let kind = error
                    .data
                    .as_ref()
                    .and_then(|data| data["code"].as_str())
                    .unwrap_or_default();
                let status = http_status_for_error_code(kind);
                // The backend's own retry hint, in whole seconds (at least 1).
                let retry_after = (status == 429).then(|| {
                    let ms = error
                        .data
                        .as_ref()
                        .and_then(|data| data["retryAfterMs"].as_u64())
                        .unwrap_or(1000);
                    format!("Retry-After: {}\r\n", ms.div_ceil(1000).max(1))
                });
                let request_header =
                    audiorouter_protocol::diagnostics::valid_correlation_id(Some(&request_id))
                        .map(|id| format!("X-Request-Id: {id}\r\n"))
                        .unwrap_or_default();
                reply_with(
                    stream,
                    status,
                    "application/json",
                    &format!("{request_header}{}", retry_after.unwrap_or_default()),
                    json!({"error": error}).to_string().as_bytes(),
                )
            }
            None => reply_with_id(
                stream,
                200,
                "application/json",
                response
                    .result
                    .unwrap_or(Value::Null)
                    .to_string()
                    .as_bytes(),
                Some(&request_id),
            ),
        },
        Err(_) => reply_with_id(
            stream,
            503,
            "application/json",
            &adapter_error_body(503, "Backend unavailable; reconnect in AudioRouter"),
            Some(&request_id),
        ),
    }
}

/// `cursor` and `limit` from the query string of `GET /api/v1/sessions`.
/// Each may appear once; anything else is refused rather than ignored.
fn paging_query(query: &str) -> Result<Value, String> {
    let mut params = serde_json::Map::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value =
            percent_decode(value).ok_or("Query values must be valid percent-encoded UTF-8")?;
        let value = match key {
            "cursor" => json!(value),
            "limit" => json!(value
                .parse::<u64>()
                .map_err(|_| "limit must be a whole number".to_owned())?),
            _ => {
                return Err(format!(
                    "Unknown query parameter {:?}; use cursor and limit",
                    key.chars().take(32).collect::<String>()
                ))
            }
        };
        if params.insert(key.to_owned(), value).is_some() {
            return Err(format!("{key} appears twice"));
        }
    }
    Ok(Value::Object(params))
}

/// Decode `%XX` escapes and `+` (a space in query strings).
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                // Two hex digits exactly: from_str_radix alone accepts "+1".
                let hex = bytes.get(index + 1..index + 3)?;
                if !hex.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                decoded.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

/// The HTTP status for a backend error code (`error.data.code`). Requests
/// the backend rejects without a code (malformed parameters) stay 400.
fn http_status_for_error_code(code: &str) -> u16 {
    match code {
        // Windows refusing a device is a permission problem too.
        "permissionDenied" | "clientRevoked" | "accessDenied" => 403,
        "notFound" => 404,
        // The request is well formed but the current state refuses it: a
        // newer revision, a reused key, an expired or too many plans, a busy
        // or exclusive device, a node not set up yet, nothing to undo.
        "revisionConflict"
        | "idempotencyConflict"
        | "planExpired"
        | "planLimitReached"
        | "deviceInUse"
        | "exclusiveModeOnly"
        | "nodeNotReady"
        | "noUndoAvailable" => 409,
        "rateLimited" => 429,
        "internalError" => 500,
        // Worth retrying later: the database, the Windows audio service, a
        // device that just went away.
        "storageFailure" | "corruptDatabase" | "serviceUnavailable" | "deviceInvalidated" => 503,
        _ => 400,
    }
}

/// The caller's `X-Request-Id` when it is a valid correlation ID; anything
/// else is replaced (never echoed or logged).
fn request_correlation_id(header: Option<&str>) -> String {
    audiorouter_protocol::diagnostics::valid_correlation_id(header)
        .map(str::to_owned)
        .unwrap_or_else(audiorouter_protocol::diagnostics::new_correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_accept::{QUEUE, WORKERS};
    use audiorouter_control::ControlPlane;

    fn exchange(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        result
    }
    fn call(api: &HttpApi, path: &str, body: Option<Value>) -> (u16, Value) {
        let body = body.map(|body| body.to_string());
        let result = exchange(api.port, &format!("{} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}", if body.is_some() { "POST" } else { "GET" }, api.port, api.token, body.as_ref().map_or(0, String::len), body.unwrap_or_default()));
        let (headers, body) = result.split_once("\r\n\r\n").unwrap();
        (
            headers.split(' ').nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(body).unwrap(),
        )
    }
    fn fixture() -> (HttpApi, Arc<Forward>) {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<(
            JsonRpcRequest,
            std::sync::mpsc::SyncSender<JsonRpcResponse>,
        )>(8);
        std::thread::spawn(move || {
            // The backend owns Windows COM objects on its original thread.
            let mut plane = ControlPlane::new("http-test");
            plane
                .insert_session(crate::default_desktop_session())
                .unwrap();
            for (request, reply) in receiver {
                let _ = reply.send(plane.dispatch(request));
            }
        });
        let forward: Arc<Forward> = Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
            let (reply, response) = std::sync::mpsc::sync_channel(1);
            sender
                .send((request.clone(), reply))
                .map_err(|_| "Backend closed")?;
            response
                .recv_timeout(Duration::from_secs(5))
                .map_err(|_| "Backend timeout".into())
        });
        (HttpApi::start(0, forward.clone()).unwrap(), forward)
    }

    #[test]
    fn a_started_request_keeps_the_normal_read_timeout() {
        let (api, _) = fixture();
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, api.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(b"G").unwrap();
        std::thread::sleep(Duration::from_millis(350));
        stream.write_all(format!("ET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n", api.port, api.token).as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    }

    #[test]
    fn silent_connections_release_workers_before_a_real_request_times_out() {
        let (api, _) = fixture();
        let silent: Vec<_> = (0..WORKERS)
            .map(|_| TcpStream::connect((Ipv4Addr::LOCALHOST, api.port)).unwrap())
            .collect();
        let started = Instant::now();
        let (status, result) = call(&api, "/api/v1/status", None);
        assert_eq!(status, 200, "{result}");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "silent clients held the workers for {:?}",
            started.elapsed()
        );
        drop(silent);
    }

    #[test]
    fn http_refuses_network_paths_before_forwarding_and_keeps_local_paths_available() {
        let (_fixture, backend) = fixture();
        let forwarded = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = Arc::clone(&forwarded);
        let api = HttpApi::start(
            0,
            Arc::new(move |request, correlation| {
                if request.method == "system.describe" {
                    return backend(request, correlation);
                }
                count.fetch_add(1, Ordering::Relaxed);
                Ok(JsonRpcResponse::success(
                    request.id.clone(),
                    json!({"forwarded": true}),
                ))
            }),
        )
        .unwrap();
        for (method, parameter) in [
            ("plugins.scan", "directory"),
            ("plugins.scan", "shellPath"),
            ("plugins.list", "directory"),
            ("plugins.retry", "directory"),
            ("plugins.inspect", "path"),
            ("plugins.parameters", "path"),
            ("plugins.buses", "path"),
            ("plugins.applyPreset", "presetPath"),
            ("sessions.importFile", "path"),
            ("nativeBridges.prepare", "renderMappingPath"),
            ("nativeBridges.prepare", "captureMappingPath"),
        ] {
            for path in [
                r"\\host\share",
                "//host/share",
                r"\\?\UNC\host\share",
                r"\\.\pipe\remote",
            ] {
                let mut params = json!({"idempotencyKey": "network-path"});
                params[parameter] = json!(path);
                let (status, error) = call(&api, &operation_path(method), Some(params));
                assert_eq!(status, 403, "{method} {path}: {error}");
                assert!(error
                    .to_string()
                    .contains("Network folders can be chosen in the AudioRouter window"));
            }
        }
        // A node's file deep inside a graph candidate or a set call was
        // forwarded with the window's grant before the generic walk.
        for (method, params) in [
            (
                "graph.plan",
                json!({"sessionId": "s", "baseRevision": 0, "candidate": {"nodes": [{"parameters": {"path": r"\\host\share\a.wav"}}]}}),
            ),
            (
                "nodes.set",
                json!({"nodeId": "file", "parameters": {"path": "//host/share/a.wav"}}),
            ),
        ] {
            let (status, error) = call(&api, &operation_path(method), Some(params));
            assert_eq!(status, 403, "{method}: {error}");
            // Adapter refusals carry a code like backend errors do, and name
            // the refused parameter.
            assert_eq!(
                error["error"]["data"]["code"], "permissionDenied",
                "{error}"
            );
            assert!(
                error["error"]["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("/parameters/path")),
                "{error}"
            );
        }
        let (status, unknown) = call(&api, "/api/v1/no/such/method", Some(json!({})));
        assert_eq!(status, 404);
        assert_eq!(unknown["error"]["data"]["code"], "notFound", "{unknown}");
        assert_eq!(forwarded.load(Ordering::Relaxed), 0);
        let (status, _) = call(
            &api,
            "/api/v1/plugins/scan",
            Some(json!({"directory": r"C:\Plugins"})),
        );
        assert_eq!(status, 200);
        assert_eq!(forwarded.load(Ordering::Relaxed), 1);
        for (method, parameter) in [
            ("plugins.buses", "path"),
            ("plugins.applyPreset", "presetPath"),
        ] {
            let mut params = json!({});
            params[parameter] = json!(r"C:\Plugins\fixture.bin");
            let (status, _) = call(&api, &operation_path(method), Some(params));
            assert_eq!(status, 200, "{method} must allow a local path");
        }
        assert_eq!(forwarded.load(Ordering::Relaxed), 3);
        let (status, schema) = call(&api, "/openapi.json", None);
        assert_eq!(status, 200);
        assert!(schema["paths"]["/api/v1/plugins/scan"].is_object());
        for method in DESKTOP_ONLY_METHODS {
            assert!(schema["paths"][operation_path(method)].is_null());
        }
    }

    #[test]
    fn every_path_parameter_has_an_http_policy_and_desktop_methods_stay_fixed() {
        let (_api, backend) = fixture();
        let describe = backend(&rpc("system.describe", None), "policy-test")
            .unwrap()
            .result
            .unwrap();
        let mut unclassified = Vec::new();
        for method in describe["methods"].as_array().unwrap() {
            let name = method["name"].as_str().unwrap();
            assert_eq!(
                matches!(http_access(name), HttpAccess::DesktopOnly(_)),
                DESKTOP_ONLY_METHODS.contains(&name),
                "{name}"
            );
            for (parameter, schema) in method["inputSchema"]["properties"].as_object().unwrap() {
                let path = matches!(
                    parameter.as_str(),
                    "path" | "directory" | "root" | "newPath"
                ) || parameter.ends_with("Path")
                    || parameter.ends_with("Directory");
                if !path || schema["type"] != "string" {
                    continue;
                }
                // This is a fixed local broker identity, not a chosen file:
                // prepare_native_bridge accepts only \\.\AudioRouterVirtualBridge.
                if name == "nativeBridges.prepare" && parameter == "devicePath" {
                    continue;
                }
                match http_access(name) {
                    HttpAccess::DesktopOnly(_) => {}
                    HttpAccess::LocalPaths(parameters)
                        if parameters.contains(&parameter.as_str()) => {}
                    // Storage confines a rename to the recording's own folder.
                    HttpAccess::Open if name == "recordings.rename" && parameter == "newPath" => {}
                    _ => unclassified.push(format!("{name}.{parameter}")),
                }
            }
        }
        assert!(
            unclassified.is_empty(),
            "Unclassified path parameters: {unclassified:?}"
        );
    }

    #[test]
    fn desktop_methods_explain_their_own_action() {
        let (api, _) = fixture();
        let mut messages = std::collections::HashSet::new();
        for method in DESKTOP_ONLY_METHODS {
            let HttpAccess::DesktopOnly(message) = http_access(method) else {
                panic!("{method}")
            };
            let (status, error) = call(&api, &operation_path(method), Some(json!({})));
            assert_eq!(status, 403);
            assert!(error.to_string().contains(message), "{method}: {error}");
            assert!(
                messages.insert(message),
                "Each action needs a specific explanation"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn drive_paths_check_the_drive_type_before_forwarding() {
        assert!(!network_path(r"C:\Plugins"));
        // Exercise DRIVE_NO_ROOT_DIR without creating a share or mapping.
        let absent = (b'A'..=b'Z')
            .find(|letter| {
                let root = [u16::from(*letter), u16::from(b':'), u16::from(b'\\'), 0];
                // SAFETY: this NUL-terminated stack buffer lives for the call;
                // GetDriveTypeW only reads it and retains no pointer.
                unsafe {
                    windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(
                        root.as_ptr(),
                    )) == 1
                }
            })
            .expect("the fixture PC has an unused drive letter");
        assert!(network_path(&format!("{}:\\Plugins", char::from(absent))));
    }
    #[test]
    fn http_plan_commit_changes_the_same_backend_and_reports_conflicts() {
        let (api, plane) = fixture();
        let (status, mut session) = call(
            &api,
            "/api/v1/sessions/get",
            Some(json!({"sessionId": "desktop-session"})),
        );
        assert_eq!(status, 200);
        session["name"] = json!("Changed over HTTP");
        let (status, plan) = call(
            &api,
            "/api/v1/graph/plan",
            Some(json!({"sessionId":"desktop-session", "baseRevision":0, "candidate":session})),
        );
        assert_eq!(status, 200, "{plan}");
        let (status, result) = call(
            &api,
            "/api/v1/graph/commit",
            Some(json!({"planId":plan["planId"],"baseRevision":0,"idempotencyKey":"http-edit"})),
        );
        assert_eq!(status, 200, "{result}");
        let read = plane(
            &rpc("sessions.get", Some(json!({"sessionId":"desktop-session"}))),
            "TEST",
        )
        .unwrap()
        .result
        .unwrap();
        assert_eq!(read["name"], "Changed over HTTP");
        assert_eq!(read["revision"], 1);
        let (status, error) = call(
            &api,
            "/api/v1/graph/plan",
            Some(json!({"sessionId":"desktop-session","baseRevision":0,"candidate":read})),
        );
        assert_eq!(status, 409, "{error}");
    }
    #[test]
    fn streamdeck_style_requests_work_by_name_in_one_call() {
        let (api, plane) = fixture();
        // A key press: toggle a node by its name, no IDs or revisions.
        let (status, toggled) = call(
            &api,
            "/api/v1/nodes/toggle",
            Some(json!({"node":"neutral GAIN","target":"bypass","idempotencyKey":"deck-1"})),
        );
        assert_eq!(status, 200, "{toggled}");
        assert_eq!(toggled["value"], true);
        let session = plane(
            &rpc("sessions.get", Some(json!({"sessionId":"desktop-session"}))),
            "TEST",
        )
        .unwrap()
        .result
        .unwrap();
        assert_eq!(
            session["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["name"] == "Neutral gain")
                .unwrap()["bypass"],
            true
        );
        // Displays read a plain summary and compact levels.
        let (status, summary) = call(&api, "/api/v1/sessions/summary", Some(json!({})));
        assert_eq!(status, 200, "{summary}");
        assert_eq!(summary["sessionId"], "desktop-session");
        let (status, levels) = call(&api, "/api/v1/meters/levels", Some(json!({})));
        assert_eq!(status, 200, "{levels}");
        assert_eq!(levels["playing"], false);
        // The recording folder is read-only here; only the window approves one.
        let (status, root) = call(&api, "/api/v1/recordings/getRoot", Some(json!({})));
        assert_eq!(status, 200, "{root}");
        let (status, refused) = call(
            &api,
            "/api/v1/recordings/setRoot",
            Some(json!({"root":"C:\\Temp","create":true,"idempotencyKey":"deck-root"})),
        );
        assert_eq!(status, 403, "{refused}");
        // A token holder cannot allow itself to open audio devices.
        let (status, refused) = call(
            &api,
            "/api/v1/devices/setAccess",
            Some(json!({"allowed":true,"idempotencyKey":"deck-devices"})),
        );
        assert_eq!(status, 403, "{refused}");
        let (status, access) = call(&api, "/api/v1/devices/getAccess", Some(json!({})));
        assert_eq!(status, 200, "{access}");
        // A token holder cannot connect to or change OBS (DEC-22).
        for (path, body) in [
            (
                "/api/v1/sessions/approveExport",
                json!({"path":"C:\\Temp\\private.audiorouter","replace":true}),
            ),
            (
                "/api/v1/sessions/exportFile",
                json!({"sessionId":"desktop-session","path":"C:\\Temp\\private.audiorouter","replace":true}),
            ),
            (
                "/api/v1/obs/connect",
                json!({"idempotencyKey":"deck-obs-1"}),
            ),
            (
                "/api/v1/obs/preview",
                json!({"changes":[{"inputName":"Mic/Aux","muted":true}]}),
            ),
            (
                "/api/v1/obs/apply",
                json!({"previewId":"obs-preview-1","idempotencyKey":"deck-obs-2"}),
            ),
            (
                "/api/v1/obs/restore",
                json!({"idempotencyKey":"deck-obs-3"}),
            ),
        ] {
            let (status, refused) = call(&api, path, Some(body));
            assert_eq!(status, 403, "{path}: {refused}");
        }
        // Mistakes come back as 400 with a readable reason.
        let (status, error) = call(
            &api,
            "/api/v1/nodes/toggle",
            Some(json!({"node":"Guitar","target":"bypass","idempotencyKey":"deck-2"})),
        );
        assert_eq!(status, 400, "{error}");
        assert!(error.to_string().contains("no node named"), "{error}");
    }
    #[test]
    fn http_rejects_token_origin_host_and_oversized_or_malformed_input() {
        let (api, _) = fixture();
        for headers in [
            format!("Host: 127.0.0.1:{}", api.port),
            format!(
                "Host: 127.0.0.1:{}\r\nAuthorization: Bearer wrong",
                api.port
            ),
        ] {
            assert!(exchange(
                api.port,
                &format!("GET /api/v1/status HTTP/1.1\r\n{headers}\r\n\r\n")
            )
            .starts_with("HTTP/1.1 401"));
        }
        for headers in [
            format!("Host: attacker.test:{}", api.port),
            format!(
                "Host: 127.0.0.1:{}\r\nOrigin: https://attacker.test",
                api.port
            ),
        ] {
            assert!(exchange(
                api.port,
                &format!(
                    "GET /api/v1/status HTTP/1.1\r\n{headers}\r\nAuthorization: Bearer {}\r\n\r\n",
                    api.token
                )
            )
            .starts_with("HTTP/1.1 403"));
        }
        let prefix = format!("POST /api/v1/graph/plan HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n", api.port, api.token);
        assert!(exchange(
            api.port,
            &format!("{prefix}Content-Length: {}\r\n\r\n", MAX_BODY + 1)
        )
        .starts_with("HTTP/1.1 413"));
        assert!(
            exchange(api.port, &format!("{prefix}Content-Length: 1\r\n\r\nx"))
                .starts_with("HTTP/1.1 400")
        );
        assert!(exchange(
            api.port,
            &format!("{prefix}Transfer-Encoding: chunked\r\n\r\n")
        )
        .starts_with("HTTP/1.1 400"));
        assert!(exchange(
            api.port,
            &format!("{prefix}Content-Length: 0\r\nContent-Length: 0\r\n\r\n")
        )
        .starts_with("HTTP/1.1 400"));
        assert!(HttpApi::start(
            api.port,
            Arc::new(|_: &JsonRpcRequest, _: &str| unreachable!())
        )
        .err()
        .unwrap()
        .contains("Choose another port"));
    }
    /// Headers are read in chunks (they were read one byte per system call):
    /// a head split across packets, with the blank line itself split, still
    /// parses; body bytes that arrive with the head are kept; a head over
    /// the limit is refused (review 2026-11-01, P3-1).
    #[test]
    fn http_reads_headers_in_chunks_and_keeps_buffered_body_bytes() {
        let (api, _) = fixture();
        let body = json!({}).to_string();
        let request = format!("POST /api/v1/sessions/summary HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", api.port, api.token, body.len());
        let split = request.find("\r\n\r\n").unwrap() + 2;
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, api.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.set_nodelay(true).unwrap();
        for part in [&request[..20], &request[20..split], &request[split..]] {
            stream.write_all(part.as_bytes()).unwrap();
            stream.flush().unwrap();
            std::thread::sleep(Duration::from_millis(30));
        }
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        assert!(result.starts_with("HTTP/1.1 200"), "{result}");
        assert!(result.contains("desktop-session"), "{result}");

        let oversized = format!(
            "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-Padding: {}\r\n\r\n",
            api.port,
            "a".repeat(MAX_HEADERS)
        );
        assert!(exchange(api.port, &oversized).starts_with("HTTP/1.1 400"));
        assert_eq!(header_end(b"GET / HTTP/1.1\r\n\r\nbody"), Some(18));
        assert_eq!(header_end(b"GET / HTTP/1.1\r\n\r"), None);
    }

    #[test]
    fn network_listener_accepts_only_private_addresses_hosts_and_peers() {
        for (address, allowed) in [
            ("192.168.1.20", true),
            ("10.0.0.5", true),
            ("172.16.0.1", true),
            ("169.254.10.2", true),
            ("8.8.8.8", false),
            ("100.64.0.1", false),
            ("0.0.0.0", false),
            ("127.0.0.1", false),
        ] {
            assert_eq!(
                lan_address_allowed(address.parse().unwrap()),
                allowed,
                "{address}"
            );
        }
        assert!(HttpApi::start_on(
            0,
            "a".repeat(64),
            Some("8.8.8.8".parse().unwrap()),
            Arc::new(|_: &JsonRpcRequest, _: &str| unreachable!())
        )
        .err()
        .unwrap()
        .contains("not a private"));
        assert!(peer_allowed("127.0.0.1".parse().unwrap(), false));
        assert!(!peer_allowed("192.168.1.30".parse().unwrap(), false));
        assert!(peer_allowed("192.168.1.30".parse().unwrap(), true));
        assert!(!peer_allowed("203.0.113.9".parse().unwrap(), true));
        assert!(peer_allowed("::ffff:192.168.1.30".parse().unwrap(), true));
        assert!(!peer_allowed("fe80::1".parse().unwrap(), true));
        let hosts = [
            "127.0.0.1:17891".to_string(),
            "192.168.1.20:17891".to_string(),
        ];
        assert!(host_allowed(Some("192.168.1.20:17891"), None, &hosts));
        assert!(host_allowed(
            Some("192.168.1.20:17891"),
            Some("http://192.168.1.20:17891"),
            &hosts
        ));
        assert!(!host_allowed(
            Some("192.168.1.20:17891"),
            Some("http://127.0.0.1:17891"),
            &hosts
        ));
        assert!(!host_allowed(Some("192.168.1.21:17891"), None, &hosts));
        assert!(!host_allowed(Some("audiorouter.local:17891"), None, &hosts));
        assert!(!host_allowed(None, None, &hosts));
    }

    /// Uses this PC's own private address when it has one; a PC with no
    /// connected private network has nothing to bind and skips.
    #[test]
    fn network_listener_serves_its_address_beside_loopback() {
        let Some(lan) = crate::lan_addresses::list()
            .unwrap()
            .first()
            .map(|entry| entry.address)
        else {
            eprintln!("no private IPv4 address on this PC; skipped");
            return;
        };
        let schema = ControlPlane::new("http-lan").describe();
        let api = HttpApi::start_on(
            0,
            "c".repeat(64),
            Some(lan),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(ControlPlane::new("http-lan").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_eq!(api.lan, Some(lan));
        assert_eq!(call(&api, "/api/v1/status", None).0, 200);
        let port = api.port;
        let over_network = |host: String| {
            let mut stream = TcpStream::connect((lan, port)).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            write!(
                stream,
                "GET /api/v1/status HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\n\r\n",
                api.token
            )
            .unwrap();
            let mut result = String::new();
            stream.read_to_string(&mut result).unwrap();
            result
        };
        assert!(over_network(format!("{lan}:{port}")).starts_with("HTTP/1.1 200"));
        assert!(over_network(format!("attacker.test:{port}")).starts_with("HTTP/1.1 403"));
        let (_, schema) = call(&api, "/openapi.json", None);
        assert_eq!(schema["servers"][1]["url"], format!("http://{lan}:{port}"));
    }

    #[test]
    fn headers_and_body_can_arrive_in_separate_packets() {
        let (api, _) = fixture();
        let mut stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, api.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "POST /api/v1/status/get HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n", api.port, api.token).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        stream.write_all(b"{").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        stream.write_all(b"}").unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        assert!(result.starts_with("HTTP/1.1 200"), "{result}");
    }
    /// A backend that answers discovery and blocks every other call until
    /// `release` is set, reporting each blocked call on `entered`.
    fn blocking_backend(
        entered: std::sync::mpsc::Sender<()>,
        release: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Arc<Forward> {
        let schema = ControlPlane::new("http-busy").describe();
        Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
            if request.method == "system.describe" {
                return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
            }
            let _ = entered.send(());
            let (open, wake) = &*release;
            let mut open = open.lock().unwrap();
            while !*open {
                open = wake.wait(open).unwrap();
            }
            Ok(ControlPlane::new("http-busy").dispatch(request.clone()))
        })
    }
    fn connect(port: u16) -> TcpStream {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
    }

    #[test]
    fn full_queue_answers_503_with_retry_after_instead_of_dropping() {
        let (entered, entered_calls) = std::sync::mpsc::channel();
        let release = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let api = HttpApi::start(0, blocking_backend(entered, release.clone())).unwrap();
        // Occupy every worker inside the backend call.
        let mut busy = Vec::new();
        for _ in 0..WORKERS {
            let mut stream = connect(api.port);
            write!(
                stream,
                "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
                api.port, api.token
            )
            .unwrap();
            busy.push(stream);
        }
        for _ in 0..WORKERS {
            entered_calls.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        // Fill the queue; nobody can take these sockets yet.
        let queued = (0..QUEUE).map(|_| connect(api.port)).collect::<Vec<_>>();
        // The next connection is answered at once rather than closed silently.
        let started = Instant::now();
        let mut overflow = connect(api.port);
        let mut response = String::new();
        overflow.read_to_string(&mut response).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(
            headers.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "{headers}"
        );
        assert!(headers.contains("\r\nRetry-After: 1\r\n"), "{headers}");
        assert!(headers.contains("\r\nConnection: close\r\n"), "{headers}");
        assert!(
            headers.contains("\r\nContent-Type: application/json\r\n"),
            "{headers}"
        );
        let body: Value = serde_json::from_str(body).unwrap();
        assert!(body["error"]["message"].as_str().unwrap().contains("busy"));
        // Releasing the backend completes the held requests normally.
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        for mut stream in busy {
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        }
        drop(queued);
        let started = Instant::now();
        drop(api);
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn backend_error_codes_get_their_own_http_status() {
        // notFound, idempotencyConflict, internalError and the database
        // failures all used to answer 400.
        for (code, status) in [
            ("permissionDenied", 403),
            ("clientRevoked", 403),
            ("notFound", 404),
            ("revisionConflict", 409),
            ("idempotencyConflict", 409),
            ("planExpired", 409),
            ("rateLimited", 429),
            ("internalError", 500),
            ("storageFailure", 503),
            ("corruptDatabase", 503),
            ("accessDenied", 403),
            ("planLimitReached", 409),
            ("deviceInUse", 409),
            ("exclusiveModeOnly", 409),
            ("nodeNotReady", 409),
            ("noUndoAvailable", 409),
            ("serviceUnavailable", 503),
            ("deviceInvalidated", 503),
            ("invalidGraph", 400),
            ("unsupportedFormat", 400),
            ("", 400),
        ] {
            assert_eq!(http_status_for_error_code(code), status, "{code}");
        }
        // Through the adapter: a missing session is 404, and a backend rate
        // limit carries its retry hint as Retry-After.
        let (_fixture, backend) = fixture();
        let api = HttpApi::start(
            0,
            Arc::new(move |request, correlation| {
                if request.method == "status.get" {
                    let mut response =
                        JsonRpcResponse::failure(request.id.clone(), -32000, "rate limited");
                    response.error.as_mut().unwrap().data =
                        Some(json!({"code": "rateLimited", "retryAfterMs": 2500}));
                    return Ok(response);
                }
                backend(request, correlation)
            }),
        )
        .unwrap();
        let (status, _) = call(
            &api,
            "/api/v1/sessions/get",
            Some(json!({"sessionId": "missing"})),
        );
        assert_eq!(status, 404);
        let raw = exchange(
            api.port,
            &format!(
                "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
                api.port, api.token
            ),
        );
        assert!(raw.starts_with("HTTP/1.1 429"), "{raw}");
        assert!(raw.contains("Retry-After: 3\r\n"), "{raw}");
    }

    #[test]
    fn get_sessions_pages_with_cursor_and_limit_query_parameters() {
        // GET took no body and refused every query string, so a script
        // could read only the first page of sessions.
        assert_eq!(
            paging_query("cursor=a%2Db+c&limit=2").unwrap(),
            json!({"cursor": "a-b c", "limit": 2})
        );
        assert!(paging_query("limit=two").is_err());
        assert!(paging_query("limit=1&limit=2").is_err());
        assert!(paging_query("sessionId=x").is_err());
        assert!(paging_query("cursor=%zz").is_err());
        assert!(paging_query("cursor=%+1").is_err());
        assert!(paging_query("cursor=%4").is_err());
        let (api, _) = fixture();
        let get = |target: &str| {
            exchange(
                api.port,
                &format!(
                    "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
                    api.port, api.token
                ),
            )
        };
        let page = get("/api/v1/sessions?limit=1");
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
        let body: Value = serde_json::from_str(page.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert!(body["items"].as_array().unwrap().len() <= 1, "{body}");
        assert!(get("/api/v1/status?verbose=1").starts_with("HTTP/1.1 400"));
        assert!(get("/api/v1/sessions?limit=x").starts_with("HTTP/1.1 400"));
    }

    #[test]
    fn openapi_covers_every_method_and_docs_are_offline() {
        let (api, plane) = fixture();
        let (_, schema) = call(&api, "/openapi.json", None);
        for spec in audiorouter_domain::API_METHODS {
            if DESKTOP_ONLY_METHODS.contains(&spec.name) {
                assert!(schema["paths"][operation_path(spec.name)].is_null());
                continue;
            }
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["operationId"],
                spec.name
            );
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["requestBody"]["content"]
                    ["application/json"]["schema"],
                plane(&rpc("system.describe", None), "TEST")
                    .unwrap()
                    .result
                    .unwrap()["methods"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|method| method["name"] == spec.name)
                    .unwrap()["inputSchema"]
            );
        }
        // Every operation documents its error responses with one body schema,
        // and the document carries the app version, not a fixed 1.0.0.
        assert_eq!(schema["info"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(schema["components"]["schemas"]["Error"].is_object());
        for (path, operations) in schema["paths"].as_object().unwrap() {
            for (verb, operation) in operations.as_object().unwrap() {
                for status in ["401", "403", "404", "409", "429", "500", "503"] {
                    assert_eq!(
                        operation["responses"][status]["content"]["application/json"]["schema"]
                            ["$ref"],
                        "#/components/schemas/Error",
                        "{verb} {path} {status}"
                    );
                }
            }
        }
        // The cable status GET alias shares its path with the POST operation.
        let cable = &schema["paths"]["/api/v1/virtualCable/status"];
        assert_eq!(cable["post"]["operationId"], "virtualCable.status");
        assert_eq!(cable["get"]["operationId"], "http.virtualCable/status");
        let docs = exchange(
            api.port,
            &format!("GET /docs HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n", api.port),
        );
        assert!(docs.contains("swagger-ui-bundle.js"));
        assert!(!docs.contains("https://"));
        let init = exchange(
            api.port,
            &format!(
                "GET /swagger-init.js HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                api.port
            ),
        );
        assert!(init.contains("validatorUrl:null"));
        assert!(init.contains("persistAuthorization:false"));
    }
    #[test]
    fn restart_preserves_token_rotation_revokes_it_and_permissions_are_preserved() {
        let schema = ControlPlane::new("http-observer").describe();
        let api = HttpApi::start(
            0,
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(plane_dispatch_observer(request))
            }),
        )
        .unwrap();
        let (status, error) = call(
            &api,
            "/api/v1/safety/setPrivacyMute",
            Some(json!({"muted":true,"idempotencyKey":"denied"})),
        );
        assert_eq!(status, 403, "{error}");
        let port = api.port;
        let token = api.token.clone();
        drop(api);
        let restarted = HttpApi::start_with_token(
            port,
            token.clone(),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                Ok(ControlPlane::new("restart").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_eq!(token, restarted.token);
        assert!(exchange(port, &format!("GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n")).starts_with("HTTP/1.1 200"));
        drop(restarted);
        let rotated = HttpApi::start_with_token(
            port,
            crate::api_token::generate().unwrap(),
            Arc::new(move |request: &JsonRpcRequest, _request_id: &str| {
                Ok(ControlPlane::new("rotation").dispatch(request.clone()))
            }),
        )
        .unwrap();
        assert_ne!(token, rotated.token);
        assert!(exchange(port, &format!("GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n")).starts_with("HTTP/1.1 401"));
        assert_eq!(call(&rotated, "/api/v1/status", None).0, 200);
    }
    #[test]
    fn http_request_ids_are_echoed_validated_and_forwarded() {
        let schema = ControlPlane::new("http-request-id").describe();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let observed = seen.clone();
        let api = HttpApi::start(
            0,
            Arc::new(move |request: &JsonRpcRequest, request_id: &str| {
                observed.lock().unwrap().push(request_id.to_owned());
                if request.method == "system.describe" {
                    return Ok(JsonRpcResponse::success(request.id.clone(), schema.clone()));
                }
                Ok(ControlPlane::new("http-request-id").dispatch(request.clone()))
            }),
        )
        .unwrap();
        let get = |extra: &str| {
            exchange(
                api.port,
                &format!(
                    "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n{extra}\r\n",
                    api.port, api.token
                ),
            )
        };
        let header = |response: &str| {
            response
                .lines()
                .find_map(|line| line.strip_prefix("X-Request-Id: "))
                .map(str::to_owned)
        };
        let echoed = get("X-Request-Id: Deck-42\r\n");
        assert!(echoed.starts_with("HTTP/1.1 200"), "{echoed}");
        assert_eq!(header(&echoed).as_deref(), Some("Deck-42"));
        assert_eq!(seen.lock().unwrap().last().unwrap(), "Deck-42");
        let long = "x".repeat(33);
        for hostile in ["has space", long.as_str(), "under_score"] {
            let replaced = get(&format!("X-Request-Id: {hostile}\r\n"));
            let id = header(&replaced).expect("a generated request ID");
            assert_ne!(id, hostile);
            assert!(audiorouter_protocol::diagnostics::is_valid_correlation_id(
                &id
            ));
            assert!(!replaced.contains(hostile), "{replaced}");
            assert_eq!(seen.lock().unwrap().last().unwrap(), &id);
        }
        let generated = header(&get("")).expect("a generated request ID");
        assert_eq!(generated.len(), 8);
        // Requests refused before reaching the backend carry no ID.
        let refused = exchange(
            api.port,
            &format!(
                "GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
                api.port
            ),
        );
        assert!(refused.starts_with("HTTP/1.1 401"));
        assert_eq!(header(&refused), None);
    }

    fn plane_dispatch_observer(request: &JsonRpcRequest) -> JsonRpcResponse {
        let mut plane = ControlPlane::new("observer");
        plane.dispatch_authorized(
            request.clone(),
            &audiorouter_control::ClientGrant::for_role(audiorouter_control::ClientRole::Observer),
        )
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "attended HTTP/browser fixture; disposable pipe and memory state, no audio"]
    fn http_browser_acceptance_fixture() {
        let pipe = format!(r"\\.\pipe\audiorouter-http-browser-{}", std::process::id());
        let server_pipe = pipe.clone();
        std::thread::spawn(move || {
            let mut plane = ControlPlane::new("http-browser");
            let mut session = crate::default_desktop_session();
            session.id = audiorouter_domain::EntityId::new("e2e-session");
            session.name = "HTTP qualification".into();
            plane.insert_session(session).unwrap();
            audiorouter_transport::serve_control_connections_forever_with_grant(
                &server_pipe,
                plane,
                audiorouter_control::ClientGrant::for_role(
                    audiorouter_control::ClientRole::Operator,
                ),
            )
            .unwrap();
        });
        let api = HttpApi::start(17893, crate::http_forward(pipe)).unwrap();
        // Explicit test-only handshake consumed in memory by the browser runner.
        // This disposable grant has no user database or audio; do not persist it.
        println!("HTTP_FIXTURE {} {}", api.port, api.token);
        for _ in 0..1800 {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
