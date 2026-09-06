# M05 visual editor evidence

## Initial React/Vite shell

The new `ui` package is a React/Vite/TypeScript application that consumes the
published local `@audiorouter/contracts` package. It provides a responsive
session sidebar, signal-flow node cards, source/effect/output library, and
recording panel. Node cards and controls are keyboard focusable, the layout
has a responsive list-friendly fallback, and the initial CSP allows only local
scripts/styles and same-origin connections.

The shell deliberately renders a disconnected/read-only state: it cannot apply
routes, access devices, or arm recording until a transport bridge is provided.
It also communicates that monitoring starts muted and recording starts
unarmed. `npm run build` passes, including TypeScript checking and Vite output;
the production-only dependency audit reports no vulnerabilities. Native shell,
transport wiring, live snapshots/events, graph editing, and accessibility
manual testing remain open.

The shell now supports local presentation-only node selection through mouse or
keyboard Enter/Space. The selected card and inspector are exposed with
accessible labels and `aria-current`; mutation controls stay disabled while
disconnected and explain that a backend connection is required. This preserves
the rule that the UI cannot duplicate or bypass graph authority. The
production build and full dependency audit pass.
