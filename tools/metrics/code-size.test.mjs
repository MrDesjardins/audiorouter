import assert from 'node:assert/strict';
import test from 'node:test';
import { classify, countLines, formatCsv, parseCsv, renderAreas, renderTotal, RULES_VERSION } from './code-size.mjs';

const area = (path) => {
  const kind = classify(path);
  if (!kind) return null;
  return kind.area === 'markdown' ? `markdown:${kind.bucket}` : `${kind.area}:${kind.test ? 'test' : 'code'}`;
};

test('classifies files into areas', () => {
  const cases = [
    ['ui/src/App.tsx', 'frontend:code'],
    ['ui/src/styles.css', 'frontend:code'],
    ['ui/src/App.accessibility.test.tsx', 'frontend:test'],
    ['ui/e2e/plugin-library.pw.ts', 'frontend:test'],
    ['ui/e2e/support/helpers.ts', 'frontend:test'],
    ['ui/route-harness.html', 'frontend:test'],
    ['ui/package.json', null],
    ['ui/dist/assets/index.js', null],
    ['crates/engine/src/lib.rs', 'backend:code'],
    ['crates/engine/tests/tool_combinations.rs', 'backend:test'],
    ['crates/control/src/graph_tests.rs', 'backend:test'],
    ['crates/control/Cargo.toml', null],
    ['src-tauri/src/main.rs', 'backend:code'],
    ['src-tauri/gen/schemas/x.rs', null],
    ['contracts/src/index.ts', 'backend:code'],
    ['contracts/node_modules/zod/index.ts', null],
    ['tests/acceptance/m02-control-route-live.ps1', 'backend:test'],
    ['tests/acceptance/m03-cable-quality.ps1', 'driver:test'],
    ['drivers/audiorouter-virtual/Source/Main/adapter.cpp', 'driver:code'],
    ['drivers/audiorouter-virtual/Source/Main/AudioRouterVirtual.inx', 'driver:code'],
    ['drivers/audiorouter-virtual/tests/bridge_tests.cpp', 'driver:test'],
    ['drivers/audiorouter-virtual/AudioRouterVirtual.sln', null],
    ['crates/driver-helper/src/lib.rs', 'driver:code'],
    ['crates/driver-helper/src/tests.rs', 'driver:test'],
    ['docs/spec/01-overview.md', 'markdown:specs'],
    ['docs/plans/active/current.md', 'markdown:specs'],
    ['README.md', 'markdown:other'],
    ['drivers/audiorouter-virtual/README.md', 'markdown:other'],
    ['docs/operations/api-reference.md', null],
    ['third_party/x/README.md', null],
    ['tools/m00-wasapi-probe/src/main.rs', 'tooling:code'],
    ['tools/m00-wasapi-probe/target/debug/build/out.rs', null],
    ['tools/quality/policy.mjs', 'tooling:code'],
    ['tools/quality/policy.test.mjs', 'tooling:test'],
    ['tools/streamdeck/src/plugin.ts', 'tooling:code'],
    ['tools/streamdeck/test/sound.test.ts', 'tooling:test'],
    ['tools/streamdeck/package.json', null],
    ['tools/vm/run-cable-tone.ps1', 'tooling:code'],
    ['tools/soundboard/README.md', 'markdown:other'],
    ['.github/workflows/ci.yml', 'tooling:code'],
    ['.githooks/pre-commit', 'tooling:code'],
    ['site/tools/gate.html', null],
  ];
  for (const [path, expected] of cases) assert.equal(area(path), expected, path);
});

test('counts non-blank lines with either line ending', () => {
  assert.deepEqual(countLines('a\r\n\r\n  \r\nb\n', false), { total: 2, test: 0 });
});

test('counts inline #[cfg(test)] modules as Rust tests', () => {
  const source = [
    'fn main() {',
    '    let s = "}"; // a brace in a string',
    "    let c = '}';",
    "    let r = r#\"{ \"json\": } }\"#;",
    '}',
    '',
    '#[cfg(test)]',
    '#[allow(clippy::unwrap_used)]',
    'mod tests {',
    '    use super::*;',
    "    fn f<'a>(x: &'a str) -> &'a str { x }",
    '    /* } */',
    '',
    '    #[test]',
    '    fn works() { assert_eq!("{", "{"); }',
    '}',
    'fn after() {}',
  ].join('\n');
  assert.deepEqual(countLines(source, true), { total: 15, test: 9 });
});

test('counts single-line test items and unterminated test modules', () => {
  assert.deepEqual(countLines('#[cfg(test)]\nmod tests;\nfn a() {}\n', true), { total: 3, test: 2 });
  assert.deepEqual(countLines('fn a() {}\n#[cfg(all(test, windows))]\nmod t {\nfn b() {}\n', true), { total: 4, test: 3 });
});

const rows = [
  { date: '2026-09-05', commit: 'a'.repeat(40), rules: RULES_VERSION, frontend_code: 0, frontend_test: 0, backend_code: 100, backend_test: 50, driver_code: 0, driver_test: 0, tooling_code: 0, tooling_test: 0, markdown_specs: 10, markdown_other: 5 },
  { date: '2026-09-06', commit: 'b'.repeat(40), rules: RULES_VERSION, frontend_code: 20, frontend_test: 10, backend_code: 150, backend_test: 70, driver_code: 30, driver_test: 4, tooling_code: 12, tooling_test: 3, markdown_specs: 40, markdown_other: 9 },
];

test('round-trips the history CSV and drops a CSV with other columns', () => {
  assert.deepEqual(parseCsv(formatCsv(rows)), rows);
  assert.deepEqual(parseCsv('date,commit\n2026-09-05,abc\n'), []);
});

test('draws five area panels and the total with direct end labels', () => {
  const areas = renderAreas(rows);
  for (const title of ['Front end', 'Back end', 'Audio driver', 'Tooling', 'Markdown']) assert.match(areas, new RegExp(title));
  assert.match(areas, /All 220</);
  assert.match(areas, /All 15</);
  assert.match(areas, /Specs &amp; plans 40</);
  const total = renderTotal(rows);
  assert.match(total, /Code 212</);
  assert.match(total, /Tests 87</);
  assert.match(total, /All 348</);
});
