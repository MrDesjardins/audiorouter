---
applies_to: **/*.rs
---

# Do not use unwrap() on fallible values in library or production code
`Option::unwrap`/`Result::unwrap` must not be used outside tests, examples, or cases where failure is provably impossible. Propagate the error with `?` or handle it explicitly.

Good:
```rust
fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let text = fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}
```

Bad:
```rust
fn load_config(path: &Path) -> Config {
    let text = fs::read_to_string(path).unwrap();
    toml::from_str(&text).unwrap()
}
```

# Prefer expect() with an invariant message over bare unwrap()
When a panic is genuinely acceptable (a broken invariant), use `expect` with a message stating *why* the value must be present, not a bare `unwrap()`.

Good:
```rust
let port: u16 = env!("DEFAULT_PORT")
    .parse()
    .expect("DEFAULT_PORT is set at build time and must be a valid u16");
```

Bad:
```rust
let port: u16 = env!("DEFAULT_PORT").parse().unwrap();
```

# Accept borrowed slices instead of owned or referenced containers
Function parameters should take `&str` instead of `&String`, `&[T]` instead of `&Vec<T>`, and `&Path` instead of `&PathBuf`, so callers are not forced into a specific owned type.

Good:
```rust
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}
```

Bad:
```rust
fn count_words(text: &String) -> usize {
    text.split_whitespace().count()
}
```

# Do not clone just to satisfy the borrow checker
Avoid `.clone()` calls whose only purpose is to silence a borrow error when a borrow, a reference, or restructuring the code would work. Unnecessary clones hide ownership design problems and cost allocations.

Good:
```rust
fn print_names(users: &[User]) {
    for user in users {
        println!("{}", user.name);
    }
}
```

Bad:
```rust
fn print_names(users: &Vec<User>) {
    for user in users.clone() {
        println!("{}", user.name.clone());
    }
}
```

# Library errors should be typed, not strings or Box<dyn Error>
Public library APIs should return a dedicated error enum (e.g. via `thiserror`) rather than `String` or `Box<dyn Error>`, so callers can match on failure cases. Application code may use `anyhow`.

Good:
```rust
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("request timed out")]
    Timeout,
    #[error("unexpected status {0}")]
    Status(u16),
}

pub fn fetch(url: &str) -> Result<Bytes, FetchError> {
    // ...
}
```

Bad:
```rust
pub fn fetch(url: &str) -> Result<Bytes, String> {
    // ...
}
```

# Add context when propagating errors in application code
When bubbling an error up in application code, attach context describing what was being attempted so the final message is actionable.

Good:
```rust
let text = fs::read_to_string(&path)
    .with_context(|| format!("reading config file {}", path.display()))?;
```

Bad:
```rust
let text = fs::read_to_string(&path)?;
```

# Do not ignore #[must_use] results
Do not discard a `Result` or other `#[must_use]` value with `let _ =` unless ignoring it is deliberate and commented.

Good:
```rust
writer.flush()?;
```

Bad:
```rust
let _ = writer.flush();
```

# Prefer iterators over index-based loops
Iterate with `for x in &items` or iterator adapters rather than `for i in 0..items.len()` with indexing, which adds bounds checks and off-by-one risk.

Good:
```rust
let total: u32 = items.iter().map(|item| item.price).sum();
```

Bad:
```rust
let mut total = 0;
for i in 0..items.len() {
    total += items[i].price;
}
```

# Every unsafe block must have a SAFETY comment
Each `unsafe` block must be preceded by a `// SAFETY:` comment explaining why the invariants the compiler cannot check are upheld.

Good:
```rust
// SAFETY: `idx` was bounds-checked against `self.len` above.
let value = unsafe { *self.ptr.add(idx) };
```

Bad:
```rust
let value = unsafe { *self.ptr.add(idx) };
```

# Keep unsafe blocks as small as possible
An `unsafe` block should wrap only the operations that actually require it, not surrounding safe code.

Good:
```rust
let len = compute_len(&input);
// SAFETY: `buf` has capacity for at least `len` bytes.
unsafe { buf.set_len(len) };
log::debug!("buffer resized to {len}");
```

Bad:
```rust
unsafe {
    let len = compute_len(&input);
    buf.set_len(len);
    log::debug!("buffer resized to {len}");
}
```

# Use the newtype pattern for domain identifiers
Wrap primitive identifiers (user IDs, order IDs) in distinct newtypes so they cannot be mixed up at call sites.

Good:
```rust
pub struct UserId(u64);
pub struct OrderId(u64);

fn cancel(user: UserId, order: OrderId) {
    // ...
}
```

Bad:
```rust
fn cancel(user: u64, order: u64) {
    // ...
}
```

# Implement From instead of Into
Provide conversions by implementing `From`; `Into` is then available automatically. Do not implement `Into` directly.

Good:
```rust
impl From<RawEvent> for Event {
    fn from(raw: RawEvent) -> Self {
        // ...
    }
}
```

Bad:
```rust
impl Into<Event> for RawEvent {
    fn into(self) -> Event {
        // ...
    }
}
```

# Do not use #[allow] to silence warnings without justification
Suppressing a lint with `#[allow(...)]` must be narrowly scoped and accompanied by a comment explaining why; never blanket `#![allow(warnings)]` at the crate root.

Good:
```rust
// The FFI struct layout mirrors a C header; field names must match.
#[allow(non_snake_case)]
struct DEVICE_INFO {
    cbSize: u32,
}
```

Bad:
```rust
#![allow(warnings)]
```

# Avoid holding a std Mutex guard across an .await
A `std::sync::MutexGuard` must not be held across an `.await` point; it makes the future `!Send` and can deadlock the executor. Drop the guard first or use an async-aware mutex.

Good:
```rust
let value = {
    let guard = state.lock().unwrap();
    guard.value.clone()
};
send(value).await;
```

Bad:
```rust
let guard = state.lock().unwrap();
send(guard.value.clone()).await;
```

# Do not call blocking operations inside async code
Async functions must not perform blocking I/O or CPU-heavy work (e.g. `std::fs`, `std::thread::sleep`) directly; use async equivalents or `spawn_blocking`.

Good:
```rust
async fn load(path: PathBuf) -> io::Result<String> {
    tokio::fs::read_to_string(path).await
}
```

Bad:
```rust
async fn load(path: PathBuf) -> io::Result<String> {
    std::thread::sleep(Duration::from_millis(100));
    std::fs::read_to_string(path)
}
```

# Prefer Rc/Arc only when shared ownership is required
Use plain ownership or borrowing by default; reach for `Rc`/`Arc` (and `RefCell`/`Mutex`) only when data genuinely needs multiple owners.

Good:
```rust
fn render(config: &Config) {
    // ...
}
```

Bad:
```rust
fn render(config: Rc<RefCell<Config>>) {
    // ...
}
```

# Use #[non_exhaustive] on public enums that may grow
Public enums and structs that may gain variants or fields in future versions should be marked `#[non_exhaustive]` to avoid breaking downstream matches.

Good:
```rust
#[non_exhaustive]
pub enum Format {
    Json,
    Yaml,
}
```

Bad:
```rust
pub enum Format {
    Json,
    Yaml,
}
```

# Prefer matching exhaustively over a wildcard arm on your own enums
When matching on an enum you control, list every variant instead of using `_ =>`, so adding a variant produces a compile error at each match site.

Good:
```rust
match status {
    Status::Active => activate(),
    Status::Suspended => suspend(),
    Status::Deleted => purge(),
}
```

Bad:
```rust
match status {
    Status::Active => activate(),
    _ => suspend(),
}
```

# Integer overflow panics in debug but wraps silently in release
Arithmetic that can overflow behaves differently in debug (panic) and release (two's-complement wrap). Use `checked_*`, `saturating_*`, or `wrapping_*` explicitly when overflow is possible.

Good:
```rust
let total = price.checked_mul(quantity).ok_or(Error::Overflow)?;
```

Bad:
```rust
let total = price * quantity;
```

# The as cast silently truncates and wraps
`as` between numeric types silently truncates, wraps, or saturates (e.g. `300_i32 as u8 == 44`, `-1_i32 as u32 == 4294967295`). Use `try_from`/`try_into` when the value may not fit.

Good:
```rust
let byte = u8::try_from(value).map_err(|_| Error::OutOfRange(value))?;
```

Bad:
```rust
let byte = value as u8;
```

# Slicing a str by byte index can panic on UTF-8 boundaries
`&s[a..b]` indexes by bytes and panics if a bound falls inside a multi-byte character. Use `char_indices`, `chars().take(n)`, or `get(a..b)` for untrusted text.

Good:
```rust
let preview: String = title.chars().take(20).collect();
```

Bad:
```rust
let preview = &title[..20];
```

# let _ = drops immediately, unlike let _guard
`let _ = lock.lock();` drops the guard at once, releasing the lock immediately; `let _guard = ...` keeps it until scope end. Bind RAII guards to a named variable.

Good:
```rust
let _guard = mutex.lock().unwrap();
update_shared_state();
```

Bad:
```rust
let _ = mutex.lock().unwrap();
update_shared_state();
```

# A RefCell double borrow panics at runtime
Calling `borrow_mut()` while another `borrow()`/`borrow_mut()` from the same `RefCell` is alive compiles fine but panics at runtime. Keep borrows short and never overlap them.

Good:
```rust
let len = cell.borrow().len();
cell.borrow_mut().push(len);
```

Bad:
```rust
let items = cell.borrow();
cell.borrow_mut().push(items.len());
```

# Rc cycles leak memory
Two `Rc` values referencing each other (e.g. parent ↔ child) are never freed. Use `Weak` for back-references.

Good:
```rust
struct Node {
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}
```

Bad:
```rust
struct Node {
    parent: RefCell<Option<Rc<Node>>>,
    children: RefCell<Vec<Rc<Node>>>,
}
```

# Iterator adapters are lazy and do nothing until consumed
Calling `map`/`filter`/`inspect` for side effects without consuming the iterator does nothing. Use a `for` loop for side effects, or consume with `collect`/`for_each`.

Good:
```rust
for path in &paths {
    fs::remove_file(path)?;
}
```

Bad:
```rust
paths.iter().map(|path| fs::remove_file(path));
```

# Futures are lazy and do nothing unless awaited or spawned
Calling an `async fn` without `.await` (or spawning it) does not run it. Every future must be awaited, joined, or spawned.

Good:
```rust
notify_subscribers(&event).await?;
```

Bad:
```rust
notify_subscribers(&event);
```

# Floating-point values must not be compared with ==
`f32`/`f64` equality is unreliable due to rounding (`0.1 + 0.2 != 0.3`), and `NaN != NaN`. Compare with a tolerance, or use integer/decimal types for money.

Good:
```rust
if (a - b).abs() < f64::EPSILON * 4.0 {
    // ...
}
```

Bad:
```rust
if a + b == 0.3 {
    // ...
}
```

# Panicking inside Drop during unwinding aborts the process
A `Drop` implementation must not panic; a panic while already unwinding aborts the whole process. Log or swallow errors in `drop`.

Good:
```rust
impl Drop for TempDir {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.path) {
            log::warn!("failed to remove {}: {e}", self.path.display());
        }
    }
}
```

Bad:
```rust
impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
```

# mem::forget and leaked guards skip destructors
`std::mem::forget` (or `Box::leak`) prevents `Drop` from running, so locks, files, and temp resources are never released. Do not use it to work around borrow problems.

Good:
```rust
drop(file);
```

Bad:
```rust
std::mem::forget(file);
```

# Do not create references to uninitialized memory
Never use `mem::uninitialized` or `mem::zeroed` for types with invalid zero/uninit states (references, `bool`, enums); this is undefined behavior. Use `MaybeUninit`.

Good:
```rust
let mut buf = MaybeUninit::<[u8; 1024]>::uninit();
```

Bad:
```rust
let buf: [u8; 1024] = unsafe { std::mem::uninitialized() };
```

# Avoid building SQL with string formatting
SQL queries must use bound parameters, never `format!` with user input.

Good:
```rust
sqlx::query("SELECT * FROM users WHERE email = $1")
    .bind(&email)
    .fetch_one(&pool)
    .await?;
```

Bad:
```rust
let sql = format!("SELECT * FROM users WHERE email = '{email}'");
sqlx::query(&sql).fetch_one(&pool).await?;
```

# Do not pass untrusted input to a shell
Run processes with `Command::new(program).arg(...)`, passing arguments individually; never build a `sh -c` string from user input.

Good:
```rust
Command::new("git")
    .args(["log", "--author", &author])
    .output()?;
```

Bad:
```rust
Command::new("sh")
    .arg("-c")
    .arg(format!("git log --author {author}"))
    .output()?;
```

# Validate file paths built from user input
Paths joined from user input must be checked to stay under the intended base directory; `Path::join` with an absolute path or `..` segments escapes it.

Good:
```rust
let candidate = base.join(name).canonicalize()?;
if !candidate.starts_with(base.canonicalize()?) {
    return Err(Error::Forbidden);
}
```

Bad:
```rust
let candidate = base.join(name);
let body = fs::read(candidate)?;
```

# Avoid disabling TLS certificate verification
Do not turn off certificate or hostname verification (e.g. `danger_accept_invalid_certs(true)`) outside of explicitly test-only code.

Good:
```rust
let client = reqwest::Client::builder().build()?;
```

Bad:
```rust
let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)
    .build()?;
```

# Compare secrets in constant time
Tokens, MACs, and password hashes must be compared with a constant-time function, not `==`, to avoid timing attacks.

Good:
```rust
use subtle::ConstantTimeEq;
let valid = provided.as_bytes().ct_eq(expected.as_bytes()).into();
```

Bad:
```rust
let valid = provided == expected;
```

# Do not log secrets via Debug
Types holding secrets (passwords, tokens, keys) must not derive `Debug` naively; implement a redacting `Debug` or wrap in a secret type so they don't leak into logs.

Good:
```rust
pub struct ApiKey(secrecy::SecretString);
```

Bad:
```rust
#[derive(Debug)]
pub struct Credentials {
    pub user: String,
    pub password: String,
}
```

# Preallocate collections with a known capacity
When the final size is known, create collections with `with_capacity` to avoid repeated reallocation.

Good:
```rust
let mut ids = Vec::with_capacity(rows.len());
for row in &rows {
    ids.push(row.id);
}
```

Bad:
```rust
let mut ids = Vec::new();
for row in &rows {
    ids.push(row.id);
}
```

# Do not collect an iterator only to iterate it again
Avoid `collect::<Vec<_>>()` followed immediately by another iteration; chain the adapters instead to skip the intermediate allocation.

Good:
```rust
let count = lines.iter().filter(|l| !l.is_empty()).count();
```

Bad:
```rust
let non_empty: Vec<_> = lines.iter().filter(|l| !l.is_empty()).collect();
let count = non_empty.len();
```

# Avoid allocating with format! or to_string in hot loops
Do not build temporary `String`s inside tight loops when you can write into a reused buffer with `write!` or `push_str`.

Good:
```rust
let mut out = String::with_capacity(items.len() * 8);
for item in &items {
    write!(out, "{},", item.id)?;
}
```

Bad:
```rust
let mut out = String::new();
for item in &items {
    out = out + &format!("{},", item.id);
}
```

# Use buffered I/O for repeated small reads and writes
Wrap `File`/`TcpStream` in `BufReader`/`BufWriter` when performing many small reads or writes; each unbuffered call is a syscall.

Good:
```rust
let mut out = BufWriter::new(File::create(path)?);
for line in &lines {
    writeln!(out, "{line}")?;
}
out.flush()?;
```

Bad:
```rust
let mut out = File::create(path)?;
for line in &lines {
    writeln!(out, "{line}")?;
}
```

# Prefer returning impl Iterator over collecting into a Vec
Functions that produce a sequence for callers to iterate should return `impl Iterator<Item = T>` rather than allocating a `Vec` the caller may not need.

Good:
```rust
fn active_users(users: &[User]) -> impl Iterator<Item = &User> {
    users.iter().filter(|u| u.active)
}
```

Bad:
```rust
fn active_users(users: &[User]) -> Vec<&User> {
    users.iter().filter(|u| u.active).collect()
}
```

# A lock taken in a match scrutinee is held for the whole match
Temporaries created in a `match` (or pre-2024 `if let`) scrutinee live until the end of the statement, so `match m.lock().unwrap().pop()` keeps the mutex locked inside every arm. Locking it again in an arm deadlocks. Bind the result to a variable first.

Good:
```rust
let next = queue.lock().unwrap().pop_front();
if let Some(job) = next {
    run(job, &queue);
}
```

Bad:
```rust
match queue.lock().unwrap().pop_front() {
    Some(job) => run(job, &queue),
    None => {}
}
```

# process::exit skips destructors
`std::process::exit` terminates immediately without running `Drop`, so buffered writers are not flushed and temp files are not cleaned up. Return an `ExitCode` from `main` instead.

Good:
```rust
fn main() -> ExitCode {
    let mut out = BufWriter::new(io::stdout());
    if let Err(e) = run(&mut out) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
```

Bad:
```rust
fn main() {
    let mut out = BufWriter::new(io::stdout());
    if let Err(e) = run(&mut out) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
```

# Dropping a BufWriter silently ignores flush errors
`BufWriter` flushes on drop but discards any error (a full disk, a closed pipe). Call `flush()` explicitly and propagate its result before the writer goes out of scope.

Good:
```rust
let mut out = BufWriter::new(File::create(path)?);
write_report(&mut out, &report)?;
out.flush()?;
```

Bad:
```rust
let mut out = BufWriter::new(File::create(path)?);
write_report(&mut out, &report)?;
```

# str::len counts bytes, not characters
`String::len`/`str::len` return the UTF-8 byte length, so `"é".len() == 2`. Use `chars().count()` (or a grapheme library) for user-visible length limits.

Good:
```rust
if username.chars().count() > 32 {
    return Err(Error::NameTooLong);
}
```

Bad:
```rust
if username.len() > 32 {
    return Err(Error::NameTooLong);
}
```

# Sorting floats with partial_cmp().unwrap() panics on NaN
`partial_cmp` returns `None` for `NaN`, so `sort_by(|a, b| a.partial_cmp(b).unwrap())` panics on the first `NaN`. Use `f64::total_cmp`.

Good:
```rust
scores.sort_by(|a, b| b.total_cmp(a));
```

Bad:
```rust
scores.sort_by(|a, b| b.partial_cmp(a).unwrap());
```

# unwrap_or evaluates its fallback eagerly
The argument to `unwrap_or`, `ok_or`, `map_or`, and `or` is evaluated even when it is not used. Use the `_else` variant when the fallback is expensive or has side effects.

Good:
```rust
let config = cached.unwrap_or_else(load_config_from_disk);
```

Bad:
```rust
let config = cached.unwrap_or(load_config_from_disk());
```

# The % operator returns negative remainders for negative operands
`%` is a remainder, not a modulo: `-7 % 3 == -1`. Use `rem_euclid` for wrap-around indices and cyclic arithmetic.

Good:
```rust
let slot = (offset + step).rem_euclid(len);
```

Bad:
```rust
let slot = (offset + step) % len;
```

# HashMap iteration order is unspecified and varies between runs
`HashMap`/`HashSet` iterate in a randomized order. Do not rely on it for output, serialization, or test assertions; use `BTreeMap` or sort first.

Good:
```rust
let counts: BTreeMap<String, usize> = tally(&words);
for (word, count) in &counts {
    writeln!(report, "{word}: {count}")?;
}
```

Bad:
```rust
let counts: HashMap<String, usize> = tally(&words);
for (word, count) in &counts {
    writeln!(report, "{word}: {count}")?;
}
```

# Vec::remove(0) is O(n); use VecDeque for queues
Removing or inserting at the front of a `Vec` shifts every element. Use `VecDeque` when items are taken from the front.

Good:
```rust
let mut queue: VecDeque<Job> = VecDeque::new();
while let Some(job) = queue.pop_front() {
    run(job);
}
```

Bad:
```rust
let mut queue: Vec<Job> = Vec::new();
while !queue.is_empty() {
    let job = queue.remove(0);
    run(job);
}
```

# A dropped JoinHandle detaches the task and hides its failures
Calling `tokio::spawn` and discarding the handle means errors returned by the task and panics inside it are silently lost, and the task may be cancelled mid-way at shutdown. Await the handle or track it in a `JoinSet`.

Good:
```rust
let handle = tokio::spawn(sync_inventory(store.clone()));
handle.await??;
```

Bad:
```rust
tokio::spawn(sync_inventory(store.clone()));
```

# env::set_var is unsound in multi-threaded programs
Modifying the process environment while other threads may read it is a data race (the 2024 edition marks `set_var`/`remove_var` as `unsafe`). Pass configuration explicitly, or set variables on a child `Command`.

Good:
```rust
Command::new("worker").env("RUST_LOG", "debug").spawn()?;
```

Bad:
```rust
std::env::set_var("RUST_LOG", "debug");
Command::new("worker").spawn()?;
```
