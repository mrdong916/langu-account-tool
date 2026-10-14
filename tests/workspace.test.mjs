import test, { after, before } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { createSSRApp, ref } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { createPinia } from 'pinia';

let server;
let WorkspaceView;
let useWorkspaceStore;
let useNativeBrowser;
const previousStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

before(async () => {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: { getItem: () => null },
  });
  server = await createServer({ server: { middlewareMode: true, hmr: false, watch: null } });
  WorkspaceView = (await server.ssrLoadModule('/src/views/workspace/WorkspaceView.vue')).default;
  ({ useWorkspaceStore } = await server.ssrLoadModule('/src/stores/workspace.ts'));
  ({ useNativeBrowser } = await server.ssrLoadModule('/src/hooks/useNativeBrowser.ts'));
});

after(async () => {
  await server?.close();
  if (previousStorage) Object.defineProperty(globalThis, 'localStorage', previousStorage);
  else delete globalThis.localStorage;
});

const account = {
  id: 'main-test',
  username: 'private.primary.username',
  realname: '测试管理员',
  avatar: 'https://example.test/avatar.png',
  department: '',
};

async function renderWorkspace({ user = account, status = 'ready', active = 'primary' } = {}) {
  const app = createSSRApp(WorkspaceView);
  const pinia = createPinia();
  app.use(pinia);
  const store = useWorkspaceStore(pinia);
  const session = {
    id: 'primary',
    role: 'primary',
    account: user,
    url: 'https://example.test/',
    title: '测试系统',
    loading: false,
    status,
    zoom: 1,
  };
  store.apply({
    settings: { ...store.state.settings, username: account.username },
    sessions: user
      ? [
          session,
          {
            ...session,
            id: 'child-test',
            role: 'secondary',
            account: { ...account, username: 'test.child', realname: '' },
          },
        ]
      : [],
    active: user ? active : null,
    access: { status: user ? status : 'pending', message: '' },
  });
  return renderToString(app);
}

test('primary profile replaces branding, shows its avatar and badge, and hides username', async () => {
  const html = await renderWorkspace();
  assert.match(html, /primary-profile/);
  assert.match(html, /测试管理员/);
  assert.match(html, /src="https:\/\/example.test\/avatar.png"/);
  assert.match(html, /avatar-primary-badge[^>]*>主<\/span>/);
  assert.equal((html.match(/avatar-primary-badge/g) ?? []).length, 1);
  assert.ok(!html.includes(account.username));
  assert.ok(!html.includes('langu-logo'));
  assert.ok(!html.includes('LANGU WORKSPACE'));
  assert.match(html, /test.child/);
});

test('username is never used as the fallback display name', async () => {
  for (const realname of ['', account.username]) {
    const html = await renderWorkspace({ user: { ...account, realname } });
    assert.match(html, /<strong>主账号<\/strong>/);
    assert.ok(!html.includes(account.username));
  }
});

test('primary profile retains identity when a child is selected', async () => {
  const html = await renderWorkspace({ active: 'child-test' });
  assert.match(html, /primary-profile[^>]*aria-pressed="false"/);
  assert.match(html, /测试管理员/);
  assert.ok(!html.includes(account.username));
});

test('signed-out state uses the avatar fallback and does not expose remembered username', async () => {
  const html = await renderWorkspace({ user: null });
  assert.match(html, /primary-profile[^>]*disabled/);
  assert.match(html, /lucide-user-round/);
  assert.match(html, /已记住主账号/);
  assert.ok(!html.includes(account.username));
});

test('expired primary status remains visible without exposing the username', async () => {
  const html = await renderWorkspace({ status: 'expired' });
  assert.match(html, /主账号登录已过期，请重新登录/);
  assert.match(html, /class="(?=[^"]*\bstatus-dot\b)(?=[^"]*\bpending\b)[^"]*"/);
  assert.ok(!html.includes(account.username));
});

test('add-account opens search only for an authorized primary, without separate navigation entries', async () => {
  const ready = await renderWorkspace();
  assert.match(ready, /aria-label="添加子账号"(?! disabled)/);
  assert.ok(!ready.includes('lucide-database'));
  assert.ok(!ready.includes('aria-label="搜索账号"'));
  for (const status of ['pending', 'checking', 'denied', 'expired', 'unavailable']) {
    const html = await renderWorkspace({ status });
    assert.match(html, /aria-label="添加子账号" disabled/);
  }
});

test('native browser restores without animation frames and skips unchanged bounds', async () => {
  const calls = [];
  const globals = {
    window: {
      __TAURI_INTERNALS__: { invoke: async (command, args) => calls.push({ command, args }) },
    },
    requestAnimationFrame: () => 0, // Simulate a background WebView that never draws a frame.
    cancelAnimationFrame: () => {},
  };
  const previous = Object.fromEntries(
    Object.keys(globals).map((key) => [key, Object.getOwnPropertyDescriptor(globalThis, key)]),
  );
  try {
    for (const [key, value] of Object.entries(globals))
      Object.defineProperty(globalThis, key, { configurable: true, value });
    const bounds = { x: 278, y: 57, width: 822, height: 671 };
    const host = ref({ getBoundingClientRect: () => bounds });
    const visible = ref(false);
    let browser;
    await renderToString(
      createSSRApp({
        setup() {
          browser = useNativeBrowser(host, visible, (error) => {
            throw error;
          });
          return () => null;
        },
      }),
    );
    browser.update();
    visible.value = true;
    browser.update();
    browser.update();
    await new Promise(setImmediate);
    assert.deepEqual(calls, [
      { command: 'browser_bounds', args: { bounds, visible: false } },
      { command: 'browser_bounds', args: { bounds, visible: true } },
    ]);
  } finally {
    for (const [key, descriptor] of Object.entries(previous)) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor);
      else delete globalThis[key];
    }
  }
});
