import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(import.meta.url);
const platforms = { darwin: 'mac', win32: 'win', linux: 'linux' };
const installerExtensions = {
  dmg: '.dmg',
  nsis: '.exe',
  msi: '.msi',
  deb: '.deb',
  rpm: '.rpm',
  appimage: '.AppImage',
};
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
  return {
    target,
    bundles: bundles.split(','),
    args: ['build', '--target', target, '--bundles', bundles],
  };
}

function installerDirectories(target, debug) {
  const cargoDir = process.env.CARGO_TARGET_DIR
    ? path.resolve(root, 'src-tauri', process.env.CARGO_TARGET_DIR)
    : path.join(root, 'src-tauri/target');
  return {
    bundle: path.join(cargoDir, target, debug ? 'debug' : 'release', 'bundle'),
    output: path.join(root, 'work/installers', target, debug ? 'debug' : 'release'),
  };
}

export function prepareInstallers(bundle, formats) {
  // Only remove generated installer directories. Keep compilation caches and earlier exports.
  for (const format of formats) {
    if (installerExtensions[format])
      rmSync(path.join(bundle, format), { recursive: true, force: true });
  }
}

export function collectInstallers(bundle, output, formats, productName, version, target) {
  const files = [];
  for (const format of formats) {
    const extension = installerExtensions[format];
    if (!extension) continue;
    const directory = path.join(bundle, format);
    const names = readdirSync(directory).filter((name) => name.endsWith(extension));
    if (names.length !== 1) {
      throw new Error(
        `${directory} 应只有一个本次构建的 ${extension} 安装包，实际为 ${names.length} 个，已停止复制。`,
      );
    }
    files.push({
      source: path.join(directory, names[0]),
      destination: path.join(
        output,
        `${productName}_${version}_${target.split('-')[0]}${extension}`,
      ),
    });
  }
  if (!files.length) throw new Error(`未在 ${bundle} 找到安装包，请检查构建输出。`);
  mkdirSync(output, { recursive: true });
  for (const { source, destination } of files) {
    copyFileSync(source, destination);
    console.log(`安装包：${destination}`);
  }
}

function main() {
  const [mode, platform = 'auto', ...rest] = process.argv.slice(2);
  const arch = rest[0] && !rest[0].startsWith('-') ? rest.shift() : undefined;
  const dryRun = rest.includes('--dry-run');
  const extra = rest.filter((value) => value !== '--dry-run');
  const plan = buildPlan(mode, platform, arch);
  // These options would invalidate the output paths below; use the named npm scripts instead.
  if (
    extra.some(
      (arg) =>
        /^(--target|--bundles|--no-bundle|--profile|--target-dir)(=|$)/.test(arg) ||
        arg === '-b' ||
        arg === '-t',
    )
  ) {
    throw new Error(
      '请用对应的平台/架构脚本选择 target 和安装包格式；自定义构建请直接使用 npx tauri build。',
    );
  }
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
  const directories = installerDirectories(plan.target, args.includes('--debug'));
  prepareInstallers(directories.bundle, plan.bundles);
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
  const config = JSON.parse(readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
  collectInstallers(
    directories.bundle,
    directories.output,
    plan.bundles,
    config.productName,
    config.version,
    plan.target,
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
