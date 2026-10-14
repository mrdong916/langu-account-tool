import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { copyFileSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(import.meta.url);
const platforms = { darwin: 'mac', win32: 'win', linux: 'linux' };
const targets = {
  mac: {
    arm64: 'aarch64-apple-darwin',
    x64: 'x86_64-apple-darwin',
    universal: 'universal-apple-darwin',
  },
  win: {
    arm64: 'aarch64-pc-windows-msvc',
    x64: 'x86_64-pc-windows-msvc',
    ia32: 'i686-pc-windows-msvc',
  },
  linux: { arm64: 'aarch64-unknown-linux-gnu', x64: 'x86_64-unknown-linux-gnu' },
};

export function buildPlan(mode, platform, arch, host = process.platform, hostArch = process.arch) {
  platform = platform === 'auto' || !platform ? platforms[host] : platform;
  arch = arch || hostArch;
  if (platform !== platforms[host])
    throw new Error(
      `请在 ${platform} 系统执行此命令。当前是 ${platforms[host] || host}；其他平台安装包可由 GitHub Actions 构建。`,
    );
  const target = targets[platform]?.[arch];
  if (!target)
    throw new Error(
      `不支持 ${platform}/${arch}，可选架构：${Object.keys(targets[platform] || {}).join(', ')}`,
    );
  if (mode !== 'build')
    throw new Error('用法：desktop.mjs build auto|mac|win|linux [x64|arm64|ia32|universal]');
  let bundles = 'deb,rpm,appimage';
  if (platform === 'mac') bundles = 'app,dmg';
  if (platform === 'win') bundles = arch === 'arm64' ? 'nsis' : 'nsis,msi';
  return { target, args: ['build', '--target', target, '--bundles', bundles] };
}

function collectInstallers(target, debug) {
  const manifest = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
  const config = JSON.parse(readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
  const cargoDir = process.env.CARGO_TARGET_DIR
    ? path.resolve(root, 'src-tauri', process.env.CARGO_TARGET_DIR)
    : path.join(root, 'src-tauri/target');
  const bundle = path.join(cargoDir, target, debug ? 'debug' : 'release', 'bundle');
  const output = path.join(root, 'work/installers', target, debug ? 'debug' : 'release');
  const extensions = {
    dmg: '.dmg',
    nsis: '.exe',
    msi: '.msi',
    deb: '.deb',
    rpm: '.rpm',
    appimage: '.AppImage',
  };
  mkdirSync(output, { recursive: true });
  let copied = 0;
  for (const [directory, extension] of Object.entries(extensions)) {
    let names;
    try {
      names = readdirSync(path.join(bundle, directory));
    } catch (error) {
      if (error.code === 'ENOENT') continue;
      throw error;
    }
    for (const name of names.filter((name) => name.endsWith(extension))) {
      const destination = path.join(
        output,
        `${config.productName}_${manifest.version}_${target.split('-')[0]}${extension}`,
      );
      copyFileSync(path.join(bundle, directory, name), destination);
      console.log(`安装包：${destination}`);
      copied++;
    }
  }
  if (!copied) throw new Error(`未在 ${bundle} 找到安装包，请检查构建输出。`);
}

function main() {
  const [mode, platform = 'auto', ...rest] = process.argv.slice(2);
  const arch = rest[0] && !rest[0].startsWith('-') ? rest.shift() : undefined;
  const dryRun = rest.includes('--dry-run');
  const extra = rest.filter((value) => value !== '--dry-run');
  const plan = buildPlan(mode, platform, arch);
  const args = [...plan.args, ...extra];
  if (mode === 'build' && !args.includes('--')) args.push('--', '--locked');
  console.log(`tauri ${args.join(' ')}`);
  if (dryRun) return;
  if (mode === 'build') {
    const needed =
      plan.target === 'universal-apple-darwin'
        ? Object.values(targets.mac).filter((target) => !target.startsWith('universal'))
        : [plan.target];
    const missing = needed.filter((target) => {
      const library = spawnSync('rustc', ['--print', 'target-libdir', '--target', target], {
        encoding: 'utf8',
      });
      if (library.error) throw new Error('请先安装 Rust stable，并将 cargo、rustc 加入 PATH。');
      try {
        return !readdirSync(library.stdout.trim()).some((name) => name.startsWith('libstd-'));
      } catch {
        return true;
      }
    });
    if (missing.length)
      throw new Error(`缺少 Rust 目标，请先执行：rustup target add ${missing.join(' ')}`);
  }
  const result = spawnSync(
    process.execPath,
    [require.resolve('@tauri-apps/cli/tauri.js'), ...args],
    {
      cwd: root,
      stdio: 'inherit',
      env: { MACOSX_DEPLOYMENT_TARGET: '14.0', ...process.env },
    },
  );
  if (result.error) throw result.error;
  if (result.status !== 0) {
    process.exitCode = result.status || 1;
    return;
  }
  if (mode === 'build') collectInstallers(plan.target, args.includes('--debug'));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
