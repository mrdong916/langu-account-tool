import test from 'node:test';
import assert from 'node:assert/strict';
import { buildPlan, prepareInstallers, collectInstallers } from '../scripts/desktop.mjs';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

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

test('local packaging excludes old versions and refuses ambiguous installer output', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'langu-packaging-'));
  const bundle = path.join(root, 'bundle');
  const output = path.join(root, 'output');
  try {
    mkdirSync(path.join(bundle, 'nsis'), { recursive: true });
    mkdirSync(path.join(bundle, 'msi'));
    writeFileSync(path.join(bundle, 'nsis/old_0.1.9.exe'), 'old');
    writeFileSync(path.join(bundle, 'msi/previous.msi'), 'unselected');
    prepareInstallers(bundle, ['nsis']);
    assert.ok(!existsSync(path.join(bundle, 'nsis')));
    assert.ok(existsSync(path.join(bundle, 'msi/previous.msi')));
    mkdirSync(path.join(bundle, 'nsis'));
    writeFileSync(path.join(bundle, 'nsis/current_0.1.10.exe'), 'new');
    collectInstallers(
      bundle,
      output,
      ['nsis'],
      '兰谷多账号工具',
      '0.1.10',
      'x86_64-pc-windows-msvc',
    );
    const exported = path.join(output, '兰谷多账号工具_0.1.10_x86_64.exe');
    assert.equal(readFileSync(exported, 'utf8'), 'new');
    assert.ok(!existsSync(path.join(output, '兰谷多账号工具_0.1.10_x86_64.msi')));
    writeFileSync(path.join(bundle, 'nsis/unexpected.exe'), 'wrong');
    assert.throws(
      () =>
        collectInstallers(
          bundle,
          output,
          ['nsis'],
          '兰谷多账号工具',
          '0.1.10',
          'x86_64-pc-windows-msvc',
        ),
      /实际为 2 个/,
    );
    assert.equal(readFileSync(exported, 'utf8'), 'new');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
