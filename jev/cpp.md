---
applies_to: tools/**/*.cpp, tools/**/*.h, tools/**/*.hpp
---

# Do not use raw new and delete for ownership
Owning heap allocations must be managed by smart pointers or containers (`std::unique_ptr`, `std::make_unique`, `std::vector`), not raw `new`/`delete`.

Good:
```cpp
auto widget = std::make_unique<Widget>(config);
```

Bad:
```cpp
Widget* widget = new Widget(config);
// ...
delete widget;
```

# Prefer make_unique and make_shared over constructing from new
Create smart pointers with `std::make_unique`/`std::make_shared` rather than passing a `new` expression to the constructor.

Good:
```cpp
auto session = std::make_shared<Session>(socket);
```

Bad:
```cpp
std::shared_ptr<Session> session(new Session(socket));
```

# Prefer unique_ptr over shared_ptr unless ownership is shared
Default to `std::unique_ptr` for single ownership; use `std::shared_ptr` only when multiple owners genuinely need to extend the lifetime.

Good:
```cpp
class Parser {
    std::unique_ptr<Lexer> lexer_;
};
```

Bad:
```cpp
class Parser {
    std::shared_ptr<Lexer> lexer_;
};
```

# Follow the rule of zero, or else the rule of five
A class should either define none of the destructor/copy/move special members (letting members manage resources), or define/delete all five consistently.

Good:
```cpp
class Buffer {
    std::vector<std::byte> data_;
};
```

Bad:
```cpp
class Buffer {
public:
    ~Buffer() { delete[] data_; }
private:
    std::byte* data_;
};
```

# Mark single-argument constructors explicit
Constructors callable with a single argument (and conversion operators) must be `explicit` unless implicit conversion is intentional.

Good:
```cpp
class Timeout {
public:
    explicit Timeout(int milliseconds);
};
```

Bad:
```cpp
class Timeout {
public:
    Timeout(int milliseconds);
};
```

# Mark overriding virtual functions with override
Every function that overrides a base-class virtual must be marked `override` (or `final`), and should not repeat `virtual`.

Good:
```cpp
class FileSink : public Sink {
    void write(std::string_view msg) override;
};
```

Bad:
```cpp
class FileSink : public Sink {
    virtual void write(std::string_view msg);
};
```

# Polymorphic base classes need a virtual destructor
A class with virtual functions that is deleted through a base pointer must have a public virtual destructor (or a protected non-virtual one).

Good:
```cpp
class Shape {
public:
    virtual ~Shape() = default;
    virtual double area() const = 0;
};
```

Bad:
```cpp
class Shape {
public:
    virtual double area() const = 0;
};
```

# Use nullptr instead of NULL or 0
Null pointers must be written as `nullptr`, never `NULL` or `0`.

Good:
```cpp
Node* head = nullptr;
```

Bad:
```cpp
Node* head = NULL;
```

# Do not use C-style casts
Use `static_cast`, `const_cast`, `reinterpret_cast`, or `dynamic_cast` rather than C-style `(T)x` casts, which silently pick the most dangerous applicable conversion.

Good:
```cpp
auto ratio = static_cast<double>(hits) / total;
```

Bad:
```cpp
auto ratio = (double)hits / total;
```

# Pass large read-only arguments by const reference
Parameters of non-trivial types that are only read should be passed as `const T&` (or `std::string_view`/`std::span`), not by value.

Good:
```cpp
void print(const std::vector<Record>& records);
```

Bad:
```cpp
void print(std::vector<Record> records);
```

# Prefer string_view for read-only string parameters
Functions that only read a string should take `std::string_view` instead of `const std::string&` or `const char*`, so callers can pass any string type without allocation.

Good:
```cpp
bool starts_with_http(std::string_view url);
```

Bad:
```cpp
bool starts_with_http(const std::string& url);
```

# Mark member functions const when they do not modify state
Member functions that do not modify the object's observable state must be declared `const`.

Good:
```cpp
class Account {
public:
    double balance() const { return balance_; }
};
```

Bad:
```cpp
class Account {
public:
    double balance() { return balance_; }
};
```

# Do not put using namespace in headers
Headers must not contain `using namespace` directives at namespace scope; they leak into every file that includes the header.

Good:
```cpp
// config.hpp
#pragma once
#include <string>

std::string load_config();
```

Bad:
```cpp
// config.hpp
#pragma once
#include <string>
using namespace std;

string load_config();
```

# Headers must have include guards
Every header must be protected by `#pragma once` or a traditional include guard.

Good:
```cpp
#pragma once

struct Point { int x; int y; };
```

Bad:
```cpp
struct Point { int x; int y; };
```

# Prefer enum class over plain enum
Use scoped enumerations (`enum class`) to avoid name leakage and implicit conversion to integers.

Good:
```cpp
enum class Color { Red, Green, Blue };
```

Bad:
```cpp
enum Color { Red, Green, Blue };
```

# Prefer constexpr or const over #define for constants
Constants must be declared with `constexpr`/`const`, not preprocessor macros, so they are typed and scoped.

Good:
```cpp
constexpr int kMaxRetries = 3;
```

Bad:
```cpp
#define MAX_RETRIES 3
```

# Use std::array or std::vector instead of C arrays
Prefer `std::array` for fixed-size and `std::vector` for dynamic arrays over raw C arrays, which decay to pointers and lose their size.

Good:
```cpp
std::array<int, 16> histogram{};
```

Bad:
```cpp
int histogram[16];
```

# Initialize all variables at declaration
Local variables and member fields must be initialized when declared; reading an uninitialized scalar is undefined behavior.

Good:
```cpp
int count = 0;
struct Stats { int hits = 0; int misses = 0; };
```

Bad:
```cpp
int count;
struct Stats { int hits; int misses; };
```

# Use RAII for locks instead of manual lock/unlock
Mutexes must be acquired through `std::lock_guard`, `std::unique_lock`, or `std::scoped_lock`, never with manual `lock()`/`unlock()` pairs that leak on exceptions or early returns.

Good:
```cpp
std::scoped_lock lock(mutex_);
queue_.push(item);
```

Bad:
```cpp
mutex_.lock();
queue_.push(item);
mutex_.unlock();
```

# Throw by value and catch by const reference
Exceptions must be thrown as values and caught by `const&`; catching by value slices derived exceptions, and throwing pointers leaks.

Good:
```cpp
try {
    connect();
} catch (const std::exception& e) {
    log(e.what());
}
```

Bad:
```cpp
try {
    connect();
} catch (std::exception e) {
    log(e.what());
}
```

# Mark move operations and swap noexcept
Move constructors, move assignment, and `swap` should be `noexcept`; otherwise `std::vector` falls back to copying during reallocation.

Good:
```cpp
Buffer(Buffer&& other) noexcept;
Buffer& operator=(Buffer&& other) noexcept;
```

Bad:
```cpp
Buffer(Buffer&& other);
Buffer& operator=(Buffer&& other);
```

# Use [[nodiscard]] on functions whose result must be checked
Functions returning error codes, newly allocated resources, or values that are pointless to ignore should be marked `[[nodiscard]]`.

Good:
```cpp
[[nodiscard]] std::error_code save(const Document& doc);
```

Bad:
```cpp
std::error_code save(const Document& doc);
```

# Prefer std::optional over sentinel values and out-parameters
Return `std::optional<T>` for "maybe a value" results instead of sentinel values like `-1` or a `bool` plus an out-parameter.

Good:
```cpp
std::optional<User> find_user(UserId id);
```

Bad:
```cpp
bool find_user(UserId id, User* out);
```

# Returning a reference or pointer to a local is a dangling reference
Never return a reference, pointer, or `string_view` into a local variable or temporary; it dangles as soon as the function returns.

Good:
```cpp
std::string full_name(const User& u) {
    return u.first + " " + u.last;
}
```

Bad:
```cpp
std::string_view full_name(const User& u) {
    std::string name = u.first + " " + u.last;
    return name;
}
```

# A string_view must not outlive the string it refers to
Do not store a `std::string_view` (or `std::span`) pointing at a temporary or at a string that may be destroyed or reallocated before the view is used.

Good:
```cpp
std::string path = build_path(dir, file);
std::string_view ext = extension(path);
```

Bad:
```cpp
std::string_view ext = extension(build_path(dir, file));
```

# Modifying a container invalidates its iterators and references
Inserting into or erasing from a `std::vector` (and similar) can invalidate existing iterators, pointers, and references. Use the iterator returned by `erase`, or the erase-remove idiom.

Good:
```cpp
std::erase_if(items, [](const Item& i) { return i.expired; });
```

Bad:
```cpp
for (auto it = items.begin(); it != items.end(); ++it) {
    if (it->expired) items.erase(it);
}
```

# operator[] on a std::map inserts missing keys
`map[key]` default-constructs and inserts an element when the key is absent. Use `find`, `contains`, or `at` for lookups that must not insert.

Good:
```cpp
if (auto it = prices.find(sku); it != prices.end()) {
    return it->second;
}
```

Bad:
```cpp
if (prices[sku] > 0) {
    return prices[sku];
}
```

# Signed integer overflow is undefined behavior
Overflowing a signed integer is UB and the optimizer may assume it never happens. Check bounds before arithmetic or use wider/unsigned types deliberately.

Good:
```cpp
if (a > std::numeric_limits<int>::max() - b) {
    throw std::overflow_error("sum overflows");
}
int sum = a + b;
```

Bad:
```cpp
int sum = a + b;
if (sum < a) {
    throw std::overflow_error("sum overflows");
}
```

# Mixing signed and unsigned in comparisons converts negatives to huge values
Comparing a signed value with an unsigned one (e.g. `size()`) converts the signed side to unsigned, so `-1 < v.size()` is false. Use `std::ssize`, `std::cmp_less`, or consistent types.

Good:
```cpp
if (std::cmp_less(index, items.size())) { ... }
```

Bad:
```cpp
int index = -1;
if (index < items.size()) { ... }
```

# Unsigned loop counters counting down never go negative
A loop like `for (size_t i = n - 1; i >= 0; --i)` never terminates because an unsigned value is always `>= 0`, and wraps on decrement past zero.

Good:
```cpp
for (size_t i = items.size(); i-- > 0;) {
    process(items[i]);
}
```

Bad:
```cpp
for (size_t i = items.size() - 1; i >= 0; --i) {
    process(items[i]);
}
```

# Do not call virtual functions from constructors or destructors
Inside a constructor or destructor, virtual calls dispatch to the current class, not the derived override, and a pure virtual call is undefined behavior.

Good:
```cpp
auto plugin = std::make_unique<AudioPlugin>();
plugin->initialize();
```

Bad:
```cpp
Plugin::Plugin() {
    initialize(); // virtual: never reaches AudioPlugin::initialize
}
```

# Member initializers run in declaration order, not list order
Members are initialized in the order they are declared in the class, regardless of the order in the constructor's initializer list. Never initialize a member from one declared after it.

Good:
```cpp
class Range {
    int size_;
    std::vector<int> data_;
public:
    explicit Range(int n) : size_(n), data_(size_) {}
};
```

Bad:
```cpp
class Range {
    std::vector<int> data_;
    int size_;
public:
    explicit Range(int n) : size_(n), data_(size_) {}
};
```

# Do not std::move from a const object or return a local with std::move
`std::move` on a `const` object silently copies, and `return std::move(local);` prevents copy elision. Return locals by name.

Good:
```cpp
std::vector<int> build() {
    std::vector<int> out;
    out.push_back(1);
    return out;
}
```

Bad:
```cpp
std::vector<int> build() {
    std::vector<int> out;
    out.push_back(1);
    return std::move(out);
}
```

# Do not use an object after it has been moved from
After `std::move(x)` is passed to something that consumes it, `x` is in a valid but unspecified state and must not be read until reassigned.

Good:
```cpp
queue.push(std::move(message));
message = Message{};
```

Bad:
```cpp
queue.push(std::move(message));
log(message.body);
```

# Lambdas capturing by reference must not outlive their scope
A lambda that captures locals (or `this`) by reference must not be stored or run asynchronously after those locals are destroyed. Capture by value or extend lifetimes explicitly.

Good:
```cpp
pool.submit([id = request.id, self = shared_from_this()] {
    self->process(id);
});
```

Bad:
```cpp
pool.submit([&] {
    process(request.id);
});
```

# Avoid shared mutable state accessed without synchronization
Data accessed from multiple threads where at least one writes must be protected by a mutex or be `std::atomic`; a data race is undefined behavior.

Good:
```cpp
std::atomic<int> processed{0};
// in worker threads:
processed.fetch_add(1, std::memory_order_relaxed);
```

Bad:
```cpp
int processed = 0;
// in worker threads:
processed++;
```

# Join or detach every std::thread before it is destroyed
Destroying a joinable `std::thread` calls `std::terminate`. Prefer `std::jthread`, which joins automatically.

Good:
```cpp
std::jthread worker([] { run_jobs(); });
```

Bad:
```cpp
void start() {
    std::thread worker([] { run_jobs(); });
}
```

# Do not use unsafe C string functions
Avoid `strcpy`, `strcat`, `sprintf`, `gets`, and similar unbounded C functions. Use `std::string`, `std::format`, or bounded alternatives.

Good:
```cpp
std::string greeting = std::format("Hello, {}!", name);
```

Bad:
```cpp
char greeting[32];
sprintf(greeting, "Hello, %s!", name);
```

# Never pass user input as a printf format string
User-controlled data must never be the format argument to `printf`-family functions; it enables format-string attacks.

Good:
```cpp
std::printf("%s", user_message.c_str());
```

Bad:
```cpp
std::printf(user_message.c_str());
```

# Bounds-check indices derived from external input
Indices or sizes coming from untrusted input (network, files, user) must be validated before indexing or copying; `operator[]` performs no bounds check.

Good:
```cpp
if (offset + length > buffer.size()) {
    throw std::out_of_range("packet exceeds buffer");
}
std::copy_n(buffer.begin() + offset, length, out.begin());
```

Bad:
```cpp
std::memcpy(out.data(), buffer.data() + offset, length);
```

# Avoid building SQL with string concatenation
SQL queries must use prepared statements with bound parameters, never string concatenation with user input.

Good:
```cpp
sqlite3_prepare_v2(db, "SELECT * FROM users WHERE email = ?", -1, &stmt, nullptr);
sqlite3_bind_text(stmt, 1, email.c_str(), -1, SQLITE_TRANSIENT);
```

Bad:
```cpp
std::string sql = "SELECT * FROM users WHERE email = '" + email + "'";
sqlite3_exec(db, sql.c_str(), callback, nullptr, nullptr);
```

# Do not pass untrusted input to system() or popen()
Never build a shell command from user input for `system`/`popen`. Use an exec-style API with an argument vector.

Good:
```cpp
const char* argv[] = {"convert", input_path.c_str(), output_path.c_str(), nullptr};
posix_spawnp(&pid, "convert", nullptr, nullptr, const_cast<char**>(argv), environ);
```

Bad:
```cpp
std::system(("convert " + input_path + " " + output_path).c_str());
```

# Do not use rand() for security-sensitive values
`std::rand` and `std::mt19937` are not cryptographically secure. Tokens, keys, and nonces must come from an OS CSPRNG or a crypto library.

Good:
```cpp
std::array<unsigned char, 32> token{};
RAND_bytes(token.data(), token.size());
```

Bad:
```cpp
std::string token = std::to_string(std::rand());
```

# Reserve vector capacity when the final size is known
Call `reserve` before a loop of `push_back`/`emplace_back` when the number of elements is known, to avoid repeated reallocation.

Good:
```cpp
std::vector<int> ids;
ids.reserve(rows.size());
for (const auto& row : rows) ids.push_back(row.id);
```

Bad:
```cpp
std::vector<int> ids;
for (const auto& row : rows) ids.push_back(row.id);
```

# Avoid copying elements in range-based for loops
Iterate non-trivial elements by `const auto&` (or `auto&&`), not by value, to avoid a copy per iteration.

Good:
```cpp
for (const auto& order : orders) {
    total += order.amount;
}
```

Bad:
```cpp
for (auto order : orders) {
    total += order.amount;
}
```

# Avoid std::endl when a newline is enough
`std::endl` flushes the stream on every call; write `'\n'` and flush only when needed.

Good:
```cpp
for (const auto& line : lines) {
    out << line << '\n';
}
```

Bad:
```cpp
for (const auto& line : lines) {
    out << line << std::endl;
}
```

# Use std::unordered_map lookups once instead of twice
Do not check `count`/`find` and then index again; reuse the iterator from a single lookup or use `try_emplace`/`insert_or_assign`.

Good:
```cpp
if (auto it = cache.find(key); it != cache.end()) {
    return it->second;
}
return cache.emplace(key, compute(key)).first->second;
```

Bad:
```cpp
if (cache.count(key) == 0) {
    cache[key] = compute(key);
}
return cache[key];
```

# std::accumulate uses the type of its initial value
`std::accumulate(v.begin(), v.end(), 0)` sums into an `int` even when `v` holds `double`s, silently truncating each step. Pass an initial value of the correct type.

Good:
```cpp
double total = std::accumulate(prices.begin(), prices.end(), 0.0);
```

Bad:
```cpp
double total = std::accumulate(prices.begin(), prices.end(), 0);
```

# std::remove does not erase elements
`std::remove`/`std::remove_if` only move kept elements forward and return a new logical end; the container size is unchanged. Pair it with `erase`, or use `std::erase_if` (C++20).

Good:
```cpp
std::erase(tags, "");
```

Bad:
```cpp
std::remove(tags.begin(), tags.end(), "");
```

# The future returned by std::async blocks in its destructor
Discarding the `std::future` from `std::async(std::launch::async, ...)` makes the temporary's destructor wait for the task, so the "async" call runs synchronously. Keep the future and `get()` it later.

Good:
```cpp
auto upload = std::async(std::launch::async, upload_logs, path);
render_frame();
upload.get();
```

Bad:
```cpp
std::async(std::launch::async, upload_logs, path);
render_frame();
```

# Do not create a shared_ptr from this
`std::shared_ptr<T>(this)` creates a second, independent control block, causing a double delete. Inherit from `std::enable_shared_from_this` and call `shared_from_this()`.

Good:
```cpp
class Connection : public std::enable_shared_from_this<Connection> {
    void start() { loop_.post(shared_from_this()); }
};
```

Bad:
```cpp
class Connection {
    void start() { loop_.post(std::shared_ptr<Connection>(this)); }
};
```

# sizeof on an array parameter returns the pointer size
An array parameter like `int values[]` decays to a pointer, so `sizeof(values)` is the pointer size, not the array size. Pass a `std::span` or a container instead.

Good:
```cpp
double average(std::span<const int> values) {
    return std::accumulate(values.begin(), values.end(), 0.0) / values.size();
}
```

Bad:
```cpp
double average(const int values[]) {
    size_t n = sizeof(values) / sizeof(values[0]);
    return std::accumulate(values, values + n, 0.0) / n;
}
```

# string_view is not null-terminated
`std::string_view::data()` is not guaranteed to be null-terminated, so passing it to a C API expecting a `const char*` string can read past the view. Convert to `std::string` first.

Good:
```cpp
void open_log(std::string_view path) {
    std::string owned(path);
    std::FILE* f = std::fopen(owned.c_str(), "a");
}
```

Bad:
```cpp
void open_log(std::string_view path) {
    std::FILE* f = std::fopen(path.data(), "a");
}
```
