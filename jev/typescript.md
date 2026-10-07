---
applies_to: **/*.ts
---

# No explicit any type
TypeScript code must not use the `any` type. Use a specific type, `unknown`,
or a generic parameter instead.

Good:
```ts
function parseJson(text: string): unknown {
  return JSON.parse(text);
}
```

Bad:
```ts
function parseJson(text: string): any {
  return JSON.parse(text);
}
```

# No empty catch blocks
A `catch` block must not be empty. It must handle the error, log it, or
rethrow it.

Good:
```ts
try {
  await save(record);
} catch (err) {
  logger.error("Failed to save record", err);
  throw err;
}
```

Bad:
```ts
try {
  await save(record);
} catch (err) {}
```

# Avoid duplicate object keys
An object literal must not define the same key twice; the earlier one is
silently discarded and almost always a mistake.

Good:
```ts
const config = { retries: 3, timeout: 1000 };
```

Bad:
```ts
const config = { retries: 3, timeout: 500, timeout: 1000 };
```

# No shadowing of outer-scope variables
An inner variable, parameter, or function must not reuse the name of a
variable already in scope from an enclosing function or block.

Good:
```ts
function processUsers(users: User[]): void {
  for (const currentUser of users) {
    handle(currentUser);
  }
}
```

Bad:
```ts
function processUsers(users: User[]): void {
  for (const users of users) { // shadows the outer `users` parameter
    handle(users);
  }
}
```

# Off-by-one loop bounds must be verified
Loop conditions over arrays or ranges must not read or write one element
past the intended end (or stop one short). Check boundary conditions
against the actual collection length.

Good:
```ts
for (let i = 0; i < items.length; i++) {
  process(items[i]);
}
```

Bad:
```ts
for (let i = 0; i <= items.length; i++) {
  process(items[i]); // reads items[items.length] === undefined
}
```

# No implicit type coercion in comparisons
Use `===`/`!==` instead of `==`/`!=`, except for the idiomatic
`== null` check that also matches `undefined`.

Good:
```ts
if (status === "active") { ... }
if (value == null) { ... } // intentionally matches null and undefined
```

Bad:
```ts
if (status == "active") { ... }
if (count != 0) { ... }
```

# Errors must not be swallowed silently
An error must not be caught and discarded without logging, rethrowing, or
otherwise surfacing it. A `catch` that only calls something like
`ignore()` or has a comment such as `// ignore` is still a violation.

Good:
```ts
try {
  await sync();
} catch (err) {
  logger.warn("Sync failed, will retry later", err);
}
```

Bad:
```ts
try {
  await sync();
} catch (err) {
  // ignore
}
```

# Promise rejections must be handled
A Promise-returning call must either be awaited inside a try/catch, or have
a `.catch()` attached. A bare `somePromise()` with no error handling is a
violation.

Good:
```ts
sendAnalyticsEvent(event).catch((err) => logger.warn("Analytics failed", err));
```

Bad:
```ts
sendAnalyticsEvent(event);
```

# Custom errors must extend Error
A custom error class must extend the built-in `Error` (or a subclass of
it), not be a plain object or plain class.

Good:
```ts
class ValidationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ValidationError";
  }
}
```

Bad:
```ts
class ValidationError {
  constructor(public message: string) {}
}
```

# Do not throw non-Error values
`throw` must only throw `Error` instances (or subclasses), never strings,
numbers, or plain objects.

Good:
```ts
throw new Error("Invalid configuration: missing apiKey");
```

Bad:
```ts
throw "Invalid configuration: missing apiKey";
```

# Async functions must not leave unhandled rejections
An `async` function called without `await` and without a `.catch()` or
surrounding try/catch at the call site is a violation.

Good:
```ts
await refreshCache();
```

Bad:
```ts
refreshCache(); // async function called and forgotten
```

# Retry logic must have a maximum attempt limit
Any retry loop or recursive retry must have an explicit maximum number of
attempts; unbounded retries are a violation.

Good:
```ts
for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
  if (await tryConnect()) return true;
}
return false;
```

Bad:
```ts
while (true) {
  if (await tryConnect()) return true;
}
```

# Parallelizable awaits must use Promise.all
Two or more independent `await` calls in sequence that do not depend on
each other's results should be run concurrently with `Promise.all`
instead of awaited one after another.

Good:
```ts
const [user, settings] = await Promise.all([fetchUser(id), fetchSettings(id)]);
```

Bad:
```ts
const user = await fetchUser(id);
const settings = await fetchSettings(id); // independent of user, but run sequentially
```

# No floating promises
A Promise-returning expression used as a standalone statement (not
returned, awaited, or assigned) is a violation.

Good:
```ts
void logAnalytics(event);
```

Bad:
```ts
logAnalytics(event); // Promise result silently discarded, not even `void`-marked
```

# setTimeout/setInterval must be cleared
A `setTimeout` or `setInterval` started inside a function/effect with a
lifetime (e.g. a component, a class instance) must have its handle cleared
via `clearTimeout`/`clearInterval` when that lifetime ends.

Good:
```ts
useEffect(() => {
  const id = setInterval(poll, 1000);
  return () => clearInterval(id);
}, []);
```

Bad:
```ts
useEffect(() => {
  setInterval(poll, 1000); // never cleared
}, []);
```

# Long-running loops must not block the event loop
A loop performing heavy synchronous work over a large or unbounded
collection on the main thread (in a server or UI context) should yield
control periodically or move the work off the main thread.

Good:
```ts
for (const batch of chunk(items, 1000)) {
  processBatch(batch);
  await new Promise((r) => setImmediate(r));
}
```

Bad:
```ts
for (const item of millionsOfItems) {
  processHeavy(item); // blocks the event loop for the entire run
}
```

# Avoid type assertions with `as` unless justified
A type assertion (`as SomeType`) must not be used to silence a real type
mismatch. It is acceptable only when the assertion is genuinely narrowing a
known-wider type (e.g. narrowing `unknown` after a runtime check).

Good:
```ts
function isUser(x: unknown): x is User {
  return typeof x === "object" && x !== null && "id" in x;
}
if (isUser(data)) { use(data); }
```

Bad:
```ts
const user = data as User; // no runtime check, just silences the type error
```

# Avoid non-null assertion operator
The non-null assertion operator (`!`) must not be used to bypass a
legitimate possibility of `null`/`undefined`; handle the case explicitly
instead.

Good:
```ts
const user = users.get(id);
if (!user) throw new Error(`User ${id} not found`);
use(user);
```

Bad:
```ts
const user = users.get(id)!;
use(user);
```

# Discriminated unions must have exhaustive switch handling
A `switch` over a discriminated union's tag must handle every member of
the union (or have a default that fails loudly, e.g. an
exhaustiveness-check helper), so a newly added union member cannot be
silently ignored.

Good:
```ts
switch (shape.kind) {
  case "circle": return Math.PI * shape.radius ** 2;
  case "square": return shape.side ** 2;
  default: return assertNever(shape);
}
```

Bad:
```ts
switch (shape.kind) {
  case "circle": return Math.PI * shape.radius ** 2;
  // "square" silently falls through to undefined if added later
}
```

# No hardcoded credentials or API keys
Source code must not contain hardcoded passwords, API keys, tokens, or
other secrets; these must come from configuration or a secrets manager.

Good:
```ts
const apiKey = process.env.TYPESAFE_API_KEY;
```

Bad:
```ts
const apiKey = "sk_live_51Hxyz...";
```

# No use of eval or new Function
`eval()` and the `new Function(...)` constructor must not be used to
execute dynamically constructed code.

Good:
```ts
const result = JSON.parse(input);
```

Bad:
```ts
const result = eval(input);
```

# User input must be validated before use
Data coming from a user, request body, query string, or other external
input must be validated (type, range, format) before being used in logic,
storage, or output.

Good:
```ts
const age = Number(req.body.age);
if (!Number.isInteger(age) || age < 0 || age > 150) {
  return res.status(400).send("Invalid age");
}
```

Bad:
```ts
const age = req.body.age;
saveUser({ age }); // used directly with no validation
```

# SQL queries must use parameterized queries
Database queries must use parameterized queries or an ORM's query builder;
string concatenation or template literals to build SQL with untrusted
input is a violation.

Good:
```ts
db.query("SELECT * FROM users WHERE email = $1", [email]);
```

Bad:
```ts
db.query(`SELECT * FROM users WHERE email = '${email}'`);
```

# No console logging of sensitive data
`console.log` and similar calls must not print secrets, tokens,
passwords, or other sensitive user data, even for debugging.

Good:
```ts
logger.debug("Login attempt", { userId: user.id });
```

Bad:
```ts
console.log("Login attempt", { password: user.password, token });
```

# Avoid insecure random number generation for security purposes
`Math.random()` must not be used to generate tokens, IDs used for
security purposes, or cryptographic material; use a cryptographically
secure random source instead.

Good:
```ts
import { randomBytes } from "node:crypto";
const token = randomBytes(32).toString("hex");
```

Bad:
```ts
const token = Math.random().toString(36).slice(2);
```

# External URLs must be validated before navigation
A URL that comes from user input or an external source must be validated
(e.g. protocol allow-list) before being used for navigation, redirects, or
opening in a new window.

Good:
```ts
const url = new URL(redirectTarget);
if (!["https:", "http:"].includes(url.protocol)) throw new Error("Blocked redirect");
window.location.href = url.toString();
```

Bad:
```ts
window.location.href = redirectTarget; // unvalidated, could be javascript:...
```

# Dependencies must not be dynamically required from user input
A `require(...)` or dynamic `import(...)` call must not use a path derived
from user-controllable input.

Good:
```ts
const handlers = { pdf: pdfHandler, csv: csvHandler };
const handler = handlers[requestedType];
```

Bad:
```ts
const handler = require(`./handlers/${req.query.type}`);
```

# Avoid O(n^2) operations on large collections
Nested iteration over the same large collection (e.g. `.find()` inside a
`.map()` over the same array) should be replaced with a lookup structure
(e.g. a `Map`) when a linear-time approach is available.

Good:
```ts
const byId = new Map(orders.map((o) => [o.id, o]));
const enriched = ids.map((id) => byId.get(id));
```

Bad:
```ts
const enriched = ids.map((id) => orders.find((o) => o.id === id));
```

# Avoid re-creating regular expressions inside loops
A regular expression literal used inside a loop body should be hoisted
outside the loop instead of being constructed on every iteration.

Good:
```ts
const emailPattern = /^[^@]+@[^@]+$/;
for (const line of lines) {
  if (emailPattern.test(line)) matches.push(line);
}
```

Bad:
```ts
for (const line of lines) {
  if (/^[^@]+@[^@]+$/.test(line)) matches.push(line);
}
```

# Avoid synchronous file I/O on the main thread
Synchronous file system calls (e.g. `readFileSync`) must not be used on a
request-handling or UI-rendering hot path; use the asynchronous equivalent.

Good:
```ts
app.get("/config", async (req, res) => {
  const data = await readFile("config.json", "utf8");
  res.send(data);
});
```

Bad:
```ts
app.get("/config", (req, res) => {
  const data = readFileSync("config.json", "utf8");
  res.send(data);
});
```

# Tests must not depend on execution order
A test must not rely on state left behind by a previous test running
first; each test should set up and tear down its own state.

Good:
```ts
test("adds an item", () => {
  const cart = new Cart();
  cart.add(item);
  assert.equal(cart.items.length, 1);
});
```

Bad:
```ts
const cart = new Cart(); // shared across tests
test("adds an item", () => { cart.add(item); assert.equal(cart.items.length, 1); });
test("cart has one item", () => { assert.equal(cart.items.length, 1); }); // depends on prior test running first
```

# Mocks must be reset between tests
Mocks, spies, or stubs must be reset or restored between tests so that
behavior from one test cannot leak into another.

Good:
```ts
afterEach(() => {
  jest.restoreAllMocks();
});
```

Bad:
```ts
test("a", () => { jest.spyOn(api, "fetch").mockReturnValue(ok); ... });
test("b", () => { /* no reset — still returns the mocked value from test "a" */ ... });
```

# No skipped tests left committed
`it.skip`, `xit`, `describe.skip`, or equivalent must not be left in
committed code without an accompanying explanation of why it's skipped.

Good:
```ts
// Skipped: flaky on CI due to timing, tracked in JIRA-991.
it.skip("retries on timeout", () => { ... });
```

Bad:
```ts
it.skip("retries on timeout", () => { ... });
```

# Assertions must not be commented out
An assertion (e.g. `expect(...)`) must not be commented out in a test; a
disabled assertion silently weakens the test's coverage.

Good:
```ts
test("computes total", () => {
  expect(computeTotal(items)).toBe(30);
});
```

Bad:
```ts
test("computes total", () => {
  const total = computeTotal(items);
  // expect(total).toBe(30);
});
```

# Circular dependencies between modules are not allowed
A module must not import (directly or transitively) from a module that
imports back from it.

Good:
```ts
// types.ts exports shared types only
// userService.ts imports from types.ts
// orderService.ts imports from types.ts
// (no module imports back from userService.ts or orderService.ts)
```

Bad:
```ts
// userService.ts
import { getOrders } from "./orderService";

// orderService.ts
import { getUser } from "./userService"; // creates a cycle with userService.ts
```

# Do not export mutable let bindings from a module
A module should not export a `let` variable that other modules can reassign; export a function or readonly value instead.

Good:
```ts
let _count = 0;
export function increment(): number {
  return ++_count;
}
```

Bad:
```ts
export let count = 0;
```

# Avoid ts-ignore; use ts-expect-error with a reason
`@ts-ignore` must not be used to silence a type error; use `@ts-expect-error` with a comment explaining why.

Good:
```ts
// @ts-expect-error legacy API returns untyped JSON
const data = legacyFetch();
```

Bad:
```ts
// @ts-ignore
const data = legacyFetch();
```

# Do not disable strict null checks locally
Do not add file-level or line-level overrides that turn off `strictNullChecks`; fix the underlying type instead.

Good:
```ts
function greet(name: string | null): string {
  return name ?? "friend";
}
```

Bad:
```ts
// @ts-nocheck
function greet(name) { return name.toUpperCase(); }
```

# Avoid double casting through unknown
Casting a value through `unknown` twice in a row (`x as unknown as T`) is a sign the type is wrong; fix the source type instead.

Good:
```ts
function isUser(x: unknown): x is User { return typeof x === "object"; }
```

Bad:
```ts
const user = data as unknown as User;
```

# Prefer Array.isArray over instanceof Array
Use `Array.isArray(x)` rather than `x instanceof Array`, which can be unreliable across realms.

Good:
```ts
if (Array.isArray(value)) { ... }
```

Bad:
```ts
if (value instanceof Array) { ... }
```

# Avoid for-in loops over arrays
Use `for-of` or array methods (`map`, `forEach`) instead of `for-in`, which iterates enumerable properties, not just indices.

Good:
```ts
for (const item of items) { process(item); }
```

Bad:
```ts
for (const i in items) { process(items[i]); }
```

# Do not mutate function parameters
A function should not reassign or mutate its parameters; treat them as read-only inputs.

Good:
```ts
function addTax(order: Order): Order {
  return { ...order, tax: order.price * 0.1 };
}
```

Bad:
```ts
function addTax(order: Order): void {
  order.tax = order.price * 0.1;
}
```

# Do not catch and rethrow the same error unchanged
A `catch` block that only rethrows the exact same error adds no value; remove the try/catch or add real handling.

Good:
```ts
await save(record);
```

Bad:
```ts
try {
  await save(record);
} catch (err) {
  throw err;
}
```

# Do not compare floating point numbers with strict equality
Floating point comparisons should use an epsilon tolerance instead of `===`, which is unreliable for computed values.

Good:
```ts
if (Math.abs(a - b) < 1e-9) { ... }
```

Bad:
```ts
if (a === b) { ... }
```

# Avoid global mutable state
Do not use module-level mutable variables as shared state; pass state explicitly or use a scoped store.

Good:
```go
type Store struct{ items []Item }
func NewStore() *Store { return &Store{} }
```

Bad:
```go
var items []Item
```

# Do not silently ignore JSON.parse errors
A `JSON.parse` call must be wrapped in error handling that surfaces or logs a parse failure, not swallow it.

Good:
```ts
try {
  return JSON.parse(text);
} catch (err) {
  logger.error("Invalid JSON", err);
  throw err;
}
```

Bad:
```ts
try { return JSON.parse(text); } catch { return null; }
```

# Avoid unbounded array growth
An array that accumulates items over the lifetime of a process must have a cap or eviction strategy.

Good:
```ts
if (history.length > MAX_HISTORY) history.shift();
history.push(entry);
```

Bad:
```ts
history.push(entry); // never trimmed
```

# Prefer structuredClone over JSON round-tripping for deep copies
Use `structuredClone` (or an explicit deep-copy utility) instead of `JSON.parse(JSON.stringify(x))`, which silently drops functions/dates/etc.

Good:
```ts
const copy = structuredClone(original);
```

Bad:
```ts
const copy = JSON.parse(JSON.stringify(original));
```

# Avoid ambiguous single-argument Array constructor calls
`new Array(n)` with a single numeric argument creates a sparse array of length `n`; prefer `Array.from({ length: n })` or a literal.

Good:
```ts
const arr = Array.from({ length: 5 });
```

Bad:
```ts
const arr = new Array(5);
```

# Prefer explicit undefined checks for numbers that can be zero
Use `x !== undefined` rather than `if (x)` when `0` is a valid, meaningful value.

Good:
```ts
if (discount !== undefined) { applyDiscount(discount); }
```

Bad:
```ts
if (discount) { applyDiscount(discount); } // 0 is falsy but valid
```

# Do not use the delete operator on array elements
Use `splice` or filter to remove array elements; `delete arr[i]` leaves a hole instead of shrinking the array.

Good:
```ts
items.splice(index, 1);
```

Bad:
```ts
delete items[index];
```

# Do not leave debugger statements in committed code
A `debugger;` statement must not be committed; it halts execution for anyone running the code with dev tools open.

Good:
```ts
function process(order: Order): void { ... }
```

Bad:
```ts
function process(order: Order): void {
  debugger;
  ...
}
```

# Do not call process.exit inside library code
Library/utility code must not call `process.exit`; let the caller (application entry point) decide process lifecycle.

Good:
```ts
throw new Error("Fatal: cannot continue");
```

Bad:
```ts
if (!valid) { process.exit(1); }
```

# Avoid catching Error and branching on message strings
Do not use `err.message.includes(...)` for control flow; use a typed/custom error class instead.

Good:
```ts
class NotFoundError extends Error {}
catch (err) { if (err instanceof NotFoundError) { ... } }
```

Bad:
```ts
catch (err) { if (err.message.includes("not found")) { ... } }
```

# forEach does not await async callbacks
`Array.prototype.forEach` does not wait for async callbacks; the loop finishes immediately while the async work is still pending.

Good:
```ts
for (const id of ids) {
  await process(id);
}
```

Bad:
```ts
ids.forEach(async (id) => {
  await process(id); // forEach doesn't wait for this
});
console.log("done"); // logs before processing finishes
```

# Array.sort defaults to lexicographic ordering
`Array.prototype.sort()` with no comparator converts elements to strings, so numbers sort lexicographically (e.g. 10 before 2) unless a comparator is supplied.

Good:
```ts
numbers.sort((a, b) => a - b);
```

Bad:
```ts
numbers.sort(); // [10, 2, 1] instead of [1, 2, 10]
```

# NaN must be checked with Number.isNaN
`isNaN()` coerces its argument before checking, so non-numeric values that aren't actually NaN can report true. Use `Number.isNaN()`, which does not coerce.

Good:
```ts
if (Number.isNaN(value)) { ... }
```

Bad:
```ts
if (isNaN(value)) { ... } // isNaN("foo") is also true
```

# Object and array equality checks compare references, not contents
`===` (and `==`) on objects/arrays compares identity, not structural equality — two objects with identical contents are never `===` unless they're the same reference.

Good:
```ts
import isEqual from "lodash/isEqual";
if (isEqual(a, b)) { ... }
```

Bad:
```ts
if (a === b) { ... } // false even when a and b have identical contents
```

# Spreading a very large array into function arguments can overflow the stack
`fn(...hugeArray)` passes every element as a separate argument; for large arrays this can exceed the engine's argument/stack limits.

Good:
```ts
const max = hugeArray.reduce((m, x) => Math.max(m, x), -Infinity);
```

Bad:
```ts
const max = Math.max(...hugeArray); // throws for large enough arrays
```

# Destructuring a possibly-undefined value throws
Destructuring `undefined` or `null` throws a TypeError; a default only applies when the whole value is exactly `undefined`, not when a nested property is missing.

Good:
```ts
const { x } = maybeUndefined ?? {};
```

Bad:
```ts
const { x } = maybeUndefined; // throws if maybeUndefined is undefined
```

# Promise.all rejects entirely on the first failure
`Promise.all` rejects as soon as any input promise rejects, discarding the results of promises that would have succeeded. Use `Promise.allSettled` when partial failures are acceptable.

Good:
```ts
const results = await Promise.allSettled(tasks);
```

Bad:
```ts
const results = await Promise.all(tasks); // one failure loses every other result
```

# Array holes are skipped by forEach/map but not by for loops
A sparse array (e.g. from `new Array(5)` or a deleted index) has holes that `forEach`/`map`/`filter` skip entirely, but a plain `for` loop still visits them as `undefined`.

Good:
```ts
const arr = Array.from({ length: 5 }, () => 0);
```

Bad:
```ts
const arr = new Array(5);
arr.forEach((x) => console.log(x)); // never runs, array is all holes
```

# Avoid path traversal from unsanitized file paths
A file path built by concatenating user input must be validated/resolved and checked against an allowed base directory, or an attacker can escape it with `../` segments.

Good:
```ts
const safePath = path.resolve(baseDir, userFile);
if (!safePath.startsWith(baseDir)) throw new Error("Invalid path");
```

Bad:
```ts
const filePath = path.join(baseDir, req.query.file); // "../../etc/passwd" escapes baseDir
```

# Avoid prototype pollution via unchecked object merges
Recursively merging an untrusted object (e.g. from request JSON) into another without filtering `__proto__`/`constructor`/`prototype` keys can pollute `Object.prototype` for the whole process.

Good:
```ts
function safeMerge(target: object, source: Record<string, unknown>) {
  for (const key of Object.keys(source)) {
    if (key === "__proto__" || key === "constructor") continue;
    (target as any)[key] = source[key];
  }
}
```

Bad:
```ts
function merge(target: any, source: any) {
  for (const key in source) target[key] = source[key]; // "__proto__" pollutes globally
}
```

# Avoid ReDoS from user-controlled regular expressions
A regular expression built from or matched against user-controlled input can have catastrophic backtracking on crafted input, hanging the process; validate/bound input length or use a safe regex engine.

Good:
```ts
if (input.length > 200) throw new Error("Input too long");
const safe = /^[a-z0-9-]{1,50}$/i;
if (!safe.test(input)) { ... }
```

Bad:
```ts
const pattern = new RegExp(`^(${userSuppliedFragment})+$`); // attacker-controlled pattern can cause catastrophic backtracking
```

# Avoid insecure deserialization of untrusted data
Deserializing untrusted data with a mechanism that can execute code (a custom `JSON.parse` reviver invoking `eval`-like behavior, or a serialization library that reconstructs class instances) can lead to remote code execution.

Good:
```ts
const data = JSON.parse(untrustedText); // plain data, no code execution
```

Bad:
```ts
const data = JSON.parse(untrustedText, (key, value) =>
  typeof value === "string" && value.startsWith("fn:") ? eval(value.slice(3)) : value
);
```

# Avoid importing an entire library when a subpath import suffices
Importing a whole library's default export just to use one function pulls the entire bundle into your output; import the specific submodule/function when the library supports it.

Good:
```ts
import debounce from "lodash/debounce";
```

Bad:
```ts
import _ from "lodash";
_.debounce(fn, 300); // bundles all of lodash for one function
```

# Debounce or throttle expensive event handlers
A handler attached to a high-frequency event (scroll, resize, input) that does expensive work on every call should be debounced or throttled, or it can visibly degrade UI responsiveness.

Good:
```ts
window.addEventListener("resize", debounce(recomputeLayout, 150));
```

Bad:
```ts
window.addEventListener("resize", recomputeLayout); // runs on every single resize tick
```

# Avoid deep-cloning large objects when a shallow copy suffices
A deep clone walks and copies every nested value; when only the top level needs to change (immutable update pattern), a shallow copy/spread is far cheaper and usually sufficient.

Good:
```ts
const updated = { ...largeConfig, timeout: 5000 };
```

Bad:
```ts
const updated = structuredClone(largeConfig);
updated.timeout = 5000; // deep-clones a large nested object just to change one field
```
