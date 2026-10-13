import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const config = JSON.parse(
  readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
);
const cargo = JSON.parse(
  execFileSync(
    'cargo',
    [
      'metadata',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      '--no-deps',
      '--locked',
      '--format-version',
      '1',
    ],
    { encoding: 'utf8' },
  ),
);
const crate = cargo.packages.find((item) => item.name === pkg.name);

assert.equal(config.version, pkg.version, 'Tauri and package.json versions must match');
assert.equal(crate?.version, pkg.version, 'Cargo and package.json versions must match');
if (process.env.GITHUB_REF?.startsWith('refs/tags/')) {
  assert.equal(
    process.env.GITHUB_REF,
    `refs/tags/v${pkg.version}`,
    'Release tag must match the app version',
  );
}
console.log(`Release version verified: ${pkg.version}`);
