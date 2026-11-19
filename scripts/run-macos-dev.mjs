import { execFileSync, spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { chmod, copyFile, cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { basename, dirname, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

// Cargo calls this runner after each dev build. Native notifications need a real app bundle.
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const [binary, ...args] = process.argv.slice(2);
const config = JSON.parse(await readFile(resolve(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const devIdentifier = `${config.identifier}.dev`;
// LaunchServices does not resolve apps inside hidden folders such as .dev, and TCC
// needs that lookup to add a Screen Recording entry, so the bundle lives in target/.
const bundle = resolve(root, 'src-tauri/target/dev/isoboard Dev.app');
const contents = resolve(bundle, 'Contents');
const executable = resolve(contents, 'MacOS/isoboard');
const plist = resolve(contents, 'Info.plist');

function signingIdentity() {
  if (process.env.APPLE_SIGNING_IDENTITY) return process.env.APPLE_SIGNING_IDENTITY;
  // Any valid certificate gives TCC a stable identity across dev rebuilds.
  const identities = execFileSync(
    '/usr/bin/security',
    ['find-identity', '-v', '-p', 'codesigning'],
    { encoding: 'utf8' },
  );
  const identity = identities.match(/^\s*\d+\) ([A-Fa-f0-9]{40}) /m)?.[1];
  if (identity) return identity;
  console.warn(
    'No valid code-signing certificate found; using ad-hoc signing. Screen Recording permissions may not persist across rebuilds. Set APPLE_SIGNING_IDENTITY to a stable certificate to test permissions.',
  );
  return '-';
}

function devProcessIds() {
  return execFileSync('/bin/ps', ['-axo', 'pid=,comm='], { encoding: 'utf8' })
    .split('\n')
    .flatMap((line) => {
      const match = line.trim().match(/^(\d+)\s+(.+)$/);
      return match?.[2] === executable ? [Number(match[1])] : [];
    });
}

function signalDevApp(signal = 'SIGTERM') {
  for (const pid of devProcessIds()) {
    try {
      process.kill(pid, signal);
    } catch (error) {
      if (error.code !== 'ESRCH') throw error;
    }
  }
}

async function stopDevApp() {
  signalDevApp();
  const deadline = Date.now() + 5000;
  while (devProcessIds().length > 0) {
    if (Date.now() >= deadline) throw new Error('Quit isoboard Dev before continuing.');
    await delay(50);
  }
}

function registerDevBundle() {
  if (!existsSync(plist)) {
    throw new Error(
      'No built dev app was found. Build it with make dev before resetting permissions.',
    );
  }
  execFileSync(
    '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister',
    ['-f', bundle],
    { stdio: 'inherit' },
  );
}

if (binary === '--reset-permissions') {
  if (process.platform !== 'darwin')
    throw new Error('Development permission reset requires macOS.');
  try {
    await stopDevApp();
    registerDevBundle();
  } catch (error) {
    console.error(`Could not prepare isoboard Dev for permission reset: ${error.message}`);
    process.exit(1);
  }
  try {
    execFileSync('/usr/bin/tccutil', ['reset', 'All', devIdentifier], { stdio: 'inherit' });
  } catch {
    console.error(`Could not reset privacy permissions for ${devIdentifier}.`);
    console.error(
      'The dev bundle was registered. Quit isoboard Dev and retry from macOS Terminal.',
    );
    process.exit(1);
  }
  console.log(`Reset privacy permissions for ${devIdentifier}.`);
  console.log(
    'For notifications, right-click isoboard Dev in Notifications and choose Reset Notifications.',
  );
  execFileSync('/usr/bin/open', [
    'x-apple.systempreferences:com.apple.Notifications-Settings.extension',
  ]);
  console.log('Restart with make dev. If needed, choose Settings → Restart initial setup.');
  process.exit(0);
}
if (binary === '--config') {
  console.log(JSON.stringify({ identifier: devIdentifier }));
  process.exit(0);
}
if (!binary) throw new Error('Expected the development binary path from Cargo.');
// The runner env also reaches other `cargo run` calls, such as the bindings exporter; run those as-is.
if (basename(binary) !== 'isoboard') {
  const child = spawn(binary, args, { stdio: 'inherit' });
  child.on('exit', (code, signal) => process.exit(signal ? 1 : (code ?? 1)));
  await new Promise(() => {});
}
await stopDevApp();
await mkdir(dirname(executable), { recursive: true });
await mkdir(resolve(contents, 'Resources'), { recursive: true });
// Match the release bundle's resource mapping before signing the development app.
for (const [source, destination] of Object.entries(config.bundle.resources)) {
  const target = resolve(contents, 'Resources', destination);
  await mkdir(dirname(target), { recursive: true });
  await cp(resolve(root, 'src-tauri', source), target, { recursive: true });
}
await copyFile(resolve(binary), executable);
await chmod(executable, 0o755);
await copyFile(resolve(root, 'src-tauri/Info.plist'), plist);
for (const [key, value] of Object.entries({
  CFBundleIdentifier: devIdentifier,
  CFBundleName: `${config.productName} Dev`,
  CFBundleDisplayName: `${config.productName} Dev`,
  CFBundleInfoDictionaryVersion: '6.0',
  CFBundleExecutable: 'isoboard',
  CFBundlePackageType: 'APPL',
  CFBundleShortVersionString: config.version,
  CFBundleVersion: config.version,
  CFBundleIconFile: 'icon.icns',
})) {
  execFileSync('/usr/bin/plutil', ['-replace', key, '-string', value, plist]);
}
await copyFile(
  resolve(root, 'src-tauri/icons/icon.icns'),
  resolve(contents, 'Resources/icon.icns'),
);
execFileSync('/usr/bin/codesign', ['--force', '--sign', signingIdentity(), bundle], {
  stdio: 'inherit',
});
registerDevBundle();

// LaunchServices gives the app its own TCC identity instead of inheriting the editor's grants.
const stdoutPath = resolve(root, '.dev/app-stdout.log');
const stderrPath = resolve(root, '.dev/app-stderr.log');
await writeFile(stdoutPath, '');
await writeFile(stderrPath, '');
const logs = [
  { path: stdoutPath, output: process.stdout },
  { path: stderrPath, output: process.stderr },
].map(({ path, output }) => {
  const tail = spawn('/usr/bin/tail', ['-n', '+1', '-f', path], {
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  tail.stdout.pipe(output);
  return tail;
});
const launchArgs = ['-n', '-W', '--stdout', stdoutPath, '--stderr', stderrPath];
if (process.env.ISOBOARD_CONFIG_PATH) {
  launchArgs.push('--env', `ISOBOARD_CONFIG_PATH=${process.env.ISOBOARD_CONFIG_PATH}`);
}
launchArgs.push(bundle, '--args', ...args);
const app = spawn('/usr/bin/open', launchArgs, { stdio: 'inherit' });
function stopLogs() {
  logs.forEach((tail) => tail.kill());
}
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
  process.on(signal, () => {
    signalDevApp(signal);
    app.kill(signal);
  });
}
process.on('exit', stopLogs);
app.on('error', (error) => {
  console.error(error);
  process.exit(1);
});
app.on('exit', async (code, signal) => {
  // A permission restart replaces the original process while retaining the dev session.
  if (!signal && code === 0) {
    await delay(2000);
    while (devProcessIds().length > 0) await delay(250);
  }
  stopLogs();
  if (signal) {
    process.removeAllListeners(signal);
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 1);
  }
});
