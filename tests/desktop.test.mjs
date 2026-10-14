import test from 'node:test';
import assert from 'node:assert/strict';
import { buildPlan } from '../scripts/desktop.mjs';

test('packaging selects platform installers and rejects builds on the wrong host', () => {
  assert.deepEqual(buildPlan('build', 'mac', 'x64', 'darwin').args, [
    'build',
    '--target',
    'x86_64-apple-darwin',
    '--bundles',
    'app,dmg',
  ]);
  assert.equal(buildPlan('build', 'mac', 'universal', 'darwin').target, 'universal-apple-darwin');
  assert.equal(buildPlan('build', 'win', 'arm64', 'win32').args.at(-1), 'nsis');
  assert.equal(buildPlan('build', 'win', 'x64', 'win32').args.at(-1), 'nsis,msi');
  assert.equal(buildPlan('build', 'linux', 'arm64', 'linux').args.at(-1), 'deb,rpm,appimage');
  assert.throws(() => buildPlan('build', 'win', 'x64', 'darwin'), /请在 win 系统/);
  assert.throws(() => buildPlan('build', 'linux', 'universal', 'linux'), /不支持/);
});
