# Connect local Claude Cowork to AudioRouter

This connection is for supported **local Cowork** deployments on the same
Windows PC. It uses standard stdio MCP, with no OpenAI tunnel or public server.
New cloud Cowork tasks cannot use it, even when opened in the desktop app.
Organization policy may disable local MCP plugins. Anthropic documents this
distinction in its [plugin guide](https://support.claude.com/en/articles/13837440-use-plugins-in-claude)
and [Cowork architecture overview](https://support.claude.com/en/articles/14479288-claude-cowork-architecture-overview).

## Setup

1. Keep AudioRouter running and open its **MCP** tab.
2. Under **Cowork**, click **Export Cowork plugin**. The app checks its installed
   MCP command, enables read-only access for `audiorouter-cowork-local`, and
   saves a uniquely named `AudioRouter-Cowork-….zip` in Downloads. The full
   path appears in the panel. Existing grants are preserved.
3. In local Cowork, open **Customize → Plugins** and upload that ZIP. Enable
   the AudioRouter plugin in a local task. This is a Cowork plugin, not a
   remote connector URL or a Desktop chat configuration entry.
4. Ask: **“Use AudioRouter to list my sessions. Do not change anything.”**
   Check AudioRouter's MCP activity for the matching client and successful call.
5. To permit edits, expand **Client permissions** in AudioRouter's MCP tab and
   choose the role for `audiorouter-cowork-local`:
   - **Editor** builds and changes sessions: add, connect and set tools, fill
     Soundboard slots, undo, duplicate and delete sessions.
   - **Operator** can also control audio that is already playing: press and
     stop Soundboard sounds, play or pause an Audio file, move a Time Shift,
     reset a Meter, mute the microphone and stop a session.

   Start with a stopped test session. Starting audio (Play) opens your audio
   devices, which needs device administration that MCP clients never receive:
   press Play in AudioRouter yourself. Recording needs its own record scope.
   No role is universal authority.

The ZIP contains only the manifest and a README. Its exact CLI, database and
pipe paths are personal configuration for this PC; do not publish the ZIP.
It contains no executables, keys, audio or database contents. Export again if
the app moves. The plugin uses the same backend as AudioRouter; it does not
start a second audio engine.

## Problems and removing access

- No export button: use the updated desktop app with its installed MCP CLI.
- Export fails: check the backend connection and client permissions; a stale
  CLI fails the protocol preflight. No ZIP is saved when preflight fails.
- Plugin imports but tools are absent: confirm the task runs locally and local
  MCP plugins are permitted by workspace policy. A cloud task needs the
  [future remote transport](../plans/future/generic-remote-mcp.md).
- Writes denied: grant the needed role/scopes deliberately; prompts cannot
  override backend authorization.
- Remove access: revoke `audiorouter-cowork-local` in Client permissions and
  uninstall the plugin in Cowork. Deleting the ZIP alone does not revoke access.

## Evidence boundary

The exporter and real CLI protocol/Observer checks are locally testable, as is
the plugin manifest with Claude Code's official validator. These do not prove
that a particular Cowork account imports or executes the plugin. Record an
actual Cowork read, permitted write and post-revocation denial separately;
see the [active plan](../plans/active/current.md).
