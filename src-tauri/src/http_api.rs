//! Optional loopback adapter. No state or audio execution is owned here.
use audiorouter_protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX_BODY: usize = 4 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
type Forward = dyn Fn(&JsonRpcRequest) -> Result<JsonRpcResponse, String> + Send + Sync;

pub struct HttpApi {
    pub port: u16,
    pub token: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for HttpApi {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
impl HttpApi {
    pub fn start(port: u16, forward: Arc<Forward>) -> Result<Self, String> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .map_err(|_| format!("Cannot open localhost port {port}. Choose another port or stop the application using it."))?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Cannot configure API listener")?;
        let port = listener
            .local_addr()
            .map_err(|_| "Cannot read API port")?
            .port();
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|_| "Cannot securely generate API token")?;
        let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let describe = forward(&rpc("system.describe", None))?
            .result
            .ok_or("Backend discovery unavailable; reconnect before starting API")?;
        let openapi = Arc::new(openapi(&describe, port));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let secret = token.clone();
        let thread = std::thread::Builder::new()
            .name("audiorouter-http".into())
            .spawn(move || {
                // Four fixed workers and 32 queued sockets bound memory/concurrency
                // while accommodating the frontend's initial discovery/read burst.
                let (sender, receiver) = std::sync::mpsc::sync_channel::<TcpStream>(32);
                let receiver = Arc::new(Mutex::new(receiver));
                let rate = Arc::new(Mutex::new((Instant::now(), 40f64)));
                let mut workers = Vec::new();
                for _ in 0..4 {
                    let (receiver, forward, schema, token, stop, rate) = (
                        receiver.clone(),
                        forward.clone(),
                        openapi.clone(),
                        secret.clone(),
                        thread_stop.clone(),
                        rate.clone(),
                    );
                    workers.push(std::thread::spawn(move || loop {
                        let stream = {
                            let Ok(receiver) = receiver.lock() else { break };
                            receiver.recv_timeout(Duration::from_millis(100))
                        };
                        if stop.load(Ordering::Acquire) {
                            break;
                        }
                        match stream {
                            Ok(mut stream) => {
                                let _ = handle(
                                    &mut stream,
                                    port,
                                    &token,
                                    &schema,
                                    &forward,
                                    &stop,
                                    &rate,
                                );
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                            Err(_) => {}
                        }
                    }));
                }
                while !thread_stop.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, address)) if address.ip().is_loopback() => {
                            let _ = sender.try_send(stream);
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(10))
                        }
                        Err(_) => break,
                    }
                }
                drop(sender);
                for worker in workers {
                    let _ = worker.join();
                }
            })
            .map_err(|_| "Cannot start HTTP adapter")?;
        Ok(Self {
            port,
            token,
            stop,
            thread: Some(thread),
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
pub fn operation_path(name: &str) -> String {
    format!("/api/v1/{}", name.replace('.', "/"))
}
pub fn openapi(describe: &Value, port: u16) -> Value {
    let mut paths = serde_json::Map::new();
    for method in describe["methods"].as_array().into_iter().flatten() {
        let Some(name) = method["name"].as_str() else {
            continue;
        };
        paths.insert(operation_path(name), json!({"post": {
            "operationId": name, "tags": [name.split('.').next().unwrap_or("API")],
            "summary": method["description"],
            "description": format!("Permission: {}. Side effect: {}. Backend validation remains authoritative.", method["permission"], method["sideEffect"]),
            "requestBody": {"required": true, "content": {"application/json": {"schema": method["inputSchema"]}}},
            "responses": {"200": {"description": "Backend result; inspect activation for durable graph commits", "content": {"application/json": {"schema": method["outputSchema"]}}}, "400": {"description": "Invalid request"}, "403": {"description": "Permission denied"}, "409": {"description": "Revision conflict"}, "429": {"description": "Rate limited"}, "503": {"description": "Backend unavailable"}},
            "security": [{"bearerAuth": []}]
        }}));
    }
    for (path, name) in [
        ("capabilities", "system.describe"),
        ("sessions", "sessions.list"),
        ("status", "status.get"),
    ] {
        let schema = describe["methods"]
            .as_array()
            .and_then(|methods| methods.iter().find(|method| method["name"] == name))
            .map(|method| method["outputSchema"].clone())
            .unwrap_or(json!({}));
        paths.insert(format!("/api/v1/{path}"), json!({"get": {"operationId": format!("http.{path}"), "security": [{"bearerAuth": []}], "responses": {"200": {"description": "Backend result", "content": {"application/json": {"schema": schema}}}}}}));
    }
    let active_get = describe["methods"].as_array().into_iter().flatten()
        .find(|method| method["name"] == "sessions.active.get").map(|method| method["outputSchema"].clone()).unwrap_or(json!({}));
    let active_set = describe["methods"].as_array().into_iter().flatten()
        .find(|method| method["name"] == "sessions.active.set");
    let active_path = paths.entry("/api/v1/sessions/active").or_insert_with(|| json!({}));
    active_path["get"] = json!({ "operationId": "http.sessions.active", "responses": { "200": { "description": "Currently selected editing session; audio is not started", "content": { "application/json": { "schema": active_get } } } }, "security": [{"bearerAuth": []}] });
    active_path["put"] = json!({ "operationId": "http.sessions.activate", "summary": "Select the editing session without starting audio", "requestBody": { "required": true, "content": { "application/json": { "schema": active_set.map(|method| method["inputSchema"].clone()).unwrap_or(json!({})) } } }, "responses": { "200": { "description": "Selected session", "content": { "application/json": { "schema": active_set.map(|method| method["outputSchema"].clone()).unwrap_or(json!({})) } } }, "400": { "description": "Unknown session"}, "403": { "description": "Permission denied"} }, "security": [{"bearerAuth": []}] });
    json!({"openapi": "3.1.0", "info": {"title": "AudioRouter local API", "version": "1.0.0"}, "servers": [{"url": format!("http://127.0.0.1:{port}")}], "paths": paths, "components": {"securitySchemes": {"bearerAuth": {"type": "http", "scheme": "bearer"}}}})
}

fn reply(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\r\n", match status { 200 => "OK", 400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden", 404 => "Not Found", 405 => "Method Not Allowed", 409 => "Conflict", 413 => "Payload Too Large", 429 => "Too Many Requests", _ => "Service Unavailable" }, body.len())?;
    stream.write_all(body)
}
fn fail(stream: &mut TcpStream, status: u16, message: &str) -> std::io::Result<()> {
    reply(
        stream,
        status,
        "application/json",
        json!({"error": {"message": message}})
            .to_string()
            .as_bytes(),
    )?;
    // Finish the response before a bounded discard of already arriving bytes.
    // Closing a Windows socket with unread POST data can reset the connection
    // and hide the actionable 401/403 response from ordinary HTTP clients.
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
    let deadline = Instant::now();
    let mut discarded = 0usize;
    let mut buffer = [0u8; 4096];
    while discarded < MAX_BODY && deadline.elapsed() < Duration::from_millis(50) {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => discarded += count,
        }
    }
    Ok(())
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
    port: u16,
    token: &str,
    schema: &Value,
    forward: &Arc<Forward>,
    stop: &AtomicBool,
    rate: &Mutex<(Instant, f64)>,
) -> std::io::Result<()> {
    // Windows accepted sockets may inherit the listener's nonblocking mode.
    // Clients send headers/body in separate packets; bounded blocking reads
    // must wait for those packets instead of treating WouldBlock as disconnect.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let started = Instant::now();
    let mut bytes = Vec::with_capacity(1024);
    while !bytes.ends_with(b"\r\n\r\n") {
        if bytes.len() >= MAX_HEADERS || started.elapsed() > Duration::from_secs(2) {
            return fail(stream, 400, "Request headers too large or too slow");
        }
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
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
    let host = format!("127.0.0.1:{port}");
    if fields.get("host") != Some(&host.as_str())
        || fields
            .get("origin")
            .is_some_and(|origin| **origin != format!("http://{host}"))
    {
        return fail(
            stream,
            403,
            "Only exact localhost Host and same-origin requests are allowed",
        );
    }
    if fields.contains_key("transfer-encoding") || path.contains('?') {
        return fail(
            stream,
            400,
            "Chunked bodies and query strings are unsupported; use JSON parameters",
        );
    }
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
        if !fields
            .get("content-type")
            .is_some_and(|value| value.split(';').next() == Some("application/json"))
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
        let mut offset = 0;
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
        None
    };
    if stop.load(Ordering::Acquire) {
        return fail(stream, 503, "API stopped");
    }
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
            return fail(stream, 429, "Local API rate exceeded; retry later");
        }
        budget.1 -= cost;
    }
    match forward(&rpc(&method, params)) {
        Ok(response) => match response.error {
            Some(error) => {
                let kind = error
                    .data
                    .as_ref()
                    .and_then(|data| data["code"].as_str())
                    .unwrap_or_default();
                let status = match kind {
                    "permissionDenied" | "clientRevoked" => 403,
                    "revisionConflict" => 409,
                    "rateLimited" => 429,
                    _ => 400,
                };
                reply(
                    stream,
                    status,
                    "application/json",
                    json!({"error": error}).to_string().as_bytes(),
                )
            }
            None => reply(
                stream,
                200,
                "application/json",
                response
                    .result
                    .unwrap_or(Value::Null)
                    .to_string()
                    .as_bytes(),
            ),
        },
        Err(_) => fail(stream, 503, "Backend unavailable; reconnect in AudioRouter"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let forward: Arc<Forward> = Arc::new(move |request| {
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
        let read = plane(&rpc(
            "sessions.get",
            Some(json!({"sessionId":"desktop-session"})),
        ))
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
        assert!(HttpApi::start(api.port, Arc::new(|_| unreachable!()))
            .err()
            .unwrap()
            .contains("Choose another port"));
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
    #[test]
    fn openapi_covers_every_method_and_docs_are_offline() {
        let (api, plane) = fixture();
        let (_, schema) = call(&api, "/openapi.json", None);
        for spec in audiorouter_domain::API_METHODS {
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["operationId"],
                spec.name
            );
            assert_eq!(
                schema["paths"][operation_path(spec.name)]["post"]["requestBody"]["content"]
                    ["application/json"]["schema"],
                plane(&rpc("system.describe", None))
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
    fn stopping_revokes_token_and_backend_permissions_are_preserved() {
        let schema = ControlPlane::new("http-observer").describe();
        let api = HttpApi::start(
            0,
            Arc::new(move |request| {
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
        let restarted = HttpApi::start(
            port,
            Arc::new(move |request| Ok(ControlPlane::new("restart").dispatch(request.clone()))),
        )
        .unwrap();
        assert_ne!(token, restarted.token);
        assert!(exchange(port, &format!("GET /api/v1/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\r\n")).starts_with("HTTP/1.1 401"));
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
        let api = HttpApi::start(
            17893,
            Arc::new(move |request| crate::forward_rpc_request(request, &pipe)),
        )
        .unwrap();
        // Explicit test-only handshake consumed in memory by the browser runner.
        // This disposable grant has no user database or audio; do not persist it.
        println!("HTTP_FIXTURE {} {}", api.port, api.token);
        for _ in 0..1800 {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
