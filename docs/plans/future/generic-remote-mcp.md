# Generic remote MCP for ChatGPT and Cowork

## Objective

Extend the existing vendor-neutral MCP server to cloud clients such as ChatGPT
and Anthropic Cowork, while preserving local stdio for desktop clients. The
OpenAI Secure MCP Tunnel remains an optional ChatGPT transport; it is not the
MCP protocol and cannot carry Cowork traffic.

## Current state and evidence

- `audiorouter mcp serve` provides local stdio MCP and shares enrollment,
  scoped grants, backend validation and audit with other clients (AUTO-06).
- ChatGPT's Secure MCP Tunnel is an OpenAI-specific outbound relay from this
  PC. The current UI supervises that client and explains the connector handoff.
- Cloud Cowork tasks cannot launch the local stdio command; their custom
  connector needs a remotely reachable MCP endpoint. Supported **local**
  Cowork deployments can use plugin-bundled stdio servers on the Windows host.
  The owner selected that local path on 2026-10-12; see the
  [local Cowork guide](../../operations/cowork-mcp.md). Local plugin support
  does not complete this future cloud transport work.
- The existing optional HTTP API is REST, not MCP. It is not a safe remote
  MCP substitute: its LAN mode is unencrypted and intentionally bounded to a
  trusted private network (HTTP-09).
- Sources checked 2026-10-11: [OpenAI custom MCP servers](https://developers.openai.com/api/docs/guides/custom-mcp-server),
  [OpenAI Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels),
  [Anthropic remote MCP connectors for Claude and Cowork](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp),
  [Anthropic local MCP for Claude Desktop](https://support.claude.com/en/articles/10949351-getting-started-with-local-mcp-servers-on-claude-desktop).

## Architecture evaluation

The protocol and the path to the user's PC are separate problems. A generic
Streamable HTTP MCP server solves the protocol side; it does not make a
Windows PC behind NAT reachable. Cowork's remote connector calls from
Anthropic's cloud and requires a server reachable over the public internet.
ChatGPT accepts either a stable remote MCP URL or its own tunnel; its official
guide supports Streamable HTTP and SSE, plus OAuth (with current CIMD/DCR
options). OpenAI Secure MCP Tunnel remains the shortest ChatGPT-only path.

| Option | Client coverage | Setup and operating cost | Assessment |
| --- | --- | --- | --- |
| Existing local stdio MCP | Local clients that can launch a command | No relay/account setup | Keep as the default desktop integration; cannot serve ChatGPT or Cowork cloud connectors. |
| OpenAI Secure MCP Tunnel | ChatGPT only | OpenAI workspace tunnel, runtime key and ChatGPT connector setup | Keep as optional immediate ChatGPT path; cannot be the generic solution. |
| User-managed Cloudflare Tunnel or Tailscale Funnel | Potentially any client that can reach the resulting URL | Requires external account; Cloudflare publishing requires an owned domain/DNS and user setup; Funnel requires Tailscale and policy enablement. The public endpoint must still implement MCP OAuth and scoped user authorization. | Useful for developer testing, not the intended no-setup consumer flow. Both publish an internet-reachable local service; neither replaces MCP auth. |
| AudioRouter-managed remote relay | ChatGPT, Cowork and other remote MCP clients | AudioRouter must operate an internet service and account/auth lifecycle | Recommended architecture if we pursue one-click remote connections. Requires explicit decisions on hosting cost, service privacy/retention, reliability, account identity and support before implementation. |

### Recommended service boundary

If pursued, run the MCP HTTP endpoint and OAuth authorization at a managed
AudioRouter relay with a stable HTTPS URL. The Windows app opens an outbound,
authenticated TLS connection to the relay and forwards only bounded MCP
control requests to the existing local MCP/backend authorization path. It does
not open an inbound port. Remote clients complete OAuth authorization against
the relay; the relay maps each user to a distinct local MCP identity and
scopes, and the app enforces its existing grants for every tool call. Keep
audio/media, arbitrary shell commands, admin keys and reusable local secrets
out of the relay. When the app is offline, fail calls promptly instead of
queueing writes. Provide per-client revoke, relay disconnect and local app
disable controls.

This is a recommendation, not a hosting decision or a deployed service. It
best matches the user's preference for avoiding manual tunnel setup while
keeping the MCP endpoint vendor-neutral. It also adds ongoing infrastructure,
security response, billing and privacy obligations. User-managed ingress is a
test/developer fallback, not a hidden default.

Primary references checked 2026-10-11: [OpenAI custom MCP and OAuth options](https://developers.openai.com/api/docs/guides/custom-mcp-server),
[OpenAI remote MCP transport guidance](https://developers.openai.com/api/docs/guides/tools-connectors-mcp),
[Anthropic Cowork remote MCP network requirement](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp),
[Cloudflare Tunnel public hostname setup](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/routing-to-tunnel/),
[Cloudflare Access for public apps](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/self-hosted-public-app/),
[Tailscale Funnel public reachability and requirements](https://tailscale.com/docs/features/tailscale-funnel).

## Future requirements and design gates

1. Add a Streamable HTTP MCP adapter over the same versioned backend API;
   keep stdio parity and permission tests.
2. Design OAuth/client identity, enrollment, token storage/rotation, revocation,
   request bounds, rate limits, origin/host handling, audit and abuse response.
   Validate against the current MCP authorization specification and both
   vendors' connector requirements.
3. Choose how a per-user Windows app is reachable from cloud clients: a
   reviewed outbound relay, or an operated hosted service. Do not ask users to
   open router/firewall ports or expose a LAN listener to the internet.
4. Keep separate provider adapters optional. A generic MCP endpoint must not
   depend on OpenAI tunnel credentials; the OpenAI tunnel must not be described
   as Cowork support.
5. Define explicit remote-access consent, per-client scoped grants and a
   visible connected-client/revocation UI. Preserve the no-audio-payload
   boundary and test that credentials and request arguments are not logged.
6. Compare operating cost, availability, privacy, data residency, support
   burden and failure recovery before selecting relay vs hosted service.

## Acceptance gate

Do not implement public reachability until the authentication, relay/hosting,
privacy and operations design has been reviewed and scheduled. Do not use
anonymous access, static shared bearer secrets, the REST listener, or OpenAI's
tunnel as a workaround. When authorized as a milestone, require process-level
MCP protocol tests, auth/revocation/rate-limit negative tests, three-theme UI
review, and an attended ChatGPT and Cowork interoperability check. Until then,
the UI must distinguish local plugin setup from cloud connectivity and avoid
claiming a connected Cowork session merely because a plugin was exported.

## Next action

The user has directed AudioRouter to favor a generic MCP solution over
depending on OpenAI's proprietary tunnel. The local Streamable HTTP adapter and
security contract can proceed as a staged implementation milestone. The
public relay still cannot be deployed until AudioRouter has a service
domain/identity model, hosting owner/budget, privacy/retention policy and an
operations owner. Preserve this boundary: local transport work does not equal
cloud reachability or Cowork support. The current MCP flow correction is
archived in [the execution record](../archived/2026-10-11-generic-mcp-ui-flow.md).

## Rollback

Keep local stdio and the optional OpenAI tunnel independent. Disable or remove
the remote endpoint/relay without changing local client registrations,
sessions, or the existing REST API.
