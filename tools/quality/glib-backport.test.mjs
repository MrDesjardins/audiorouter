import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

function checkOutPointer(source) {
  const body = source.slice(source.indexOf("fn impl_get(&self, i: usize) -> &'a str"));
  assert.match(body, /let mut p: \*mut libc::c_char = std::ptr::null_mut\(\)/);
  assert.match(body, /ffi::g_variant_get_child\([\s\S]*?&mut p,/);
  assert.doesNotMatch(body, /\n\s*&p,/);
}

test('all shell GLib consumers resolve to the soundness backport', () => {
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--manifest-path',
    'src-tauri/Cargo.toml', '--locked', '--format-version', '1'], { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 }));
  const glib = metadata.packages.filter(p => p.name === 'glib');
  assert.equal(glib.length, 1);
  assert.equal(glib[0].source, null);
  assert.equal(resolve(glib[0].manifest_path), resolve('vendor/glib/Cargo.toml'));
  checkOutPointer(readFileSync('vendor/glib/src/variant_iter.rs', 'utf8'));
});

test('guard rejects the original immutable out-pointer', () => {
  const source = readFileSync('vendor/glib/src/variant_iter.rs', 'utf8');
  assert.throws(() => checkOutPointer(source.replace('&mut p,', '&p,')));
  assert.throws(() => checkOutPointer(source.replace('let mut p: *mut libc::c_char', 'let p: *mut libc::c_char')));
});
