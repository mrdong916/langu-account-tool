import test, { after, before } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { createSSRApp, ref, nextTick } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { createPinia } from 'pinia';

let server;
let WorkspaceView;
let useWorkspaceStore;
let useNativeBrowser;
let historyApi, Dialog;
const storage = new Map();
const previousStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

before(async () => {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value),
      removeItem: (key) => storage.delete(key),
    },
  });
  server = await createServer({ server: { middlewareMode: true, hmr: false, watch: null } });
  historyApi = await server.ssrLoadModule('/src/api/searchHistory.ts');
  Dialog = (await server.ssrLoadModule('/src/components/account/UserSearchDialog.vue')).default;
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
  for (const status of ['pending', 'away', 'checking', 'denied', 'expired', 'unavailable']) {
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

test('search history persists by owner, deduplicates, limits and recovers malformed data', () => {
  const { saveSearchHistory, readSearchHistory } = historyApi;
  saveSearchHistory('owner-a', [
    '  ZS  ',
    'zs',
    ' 财务  张 ',
    '',
    ...Array.from({ length: 15 }, (_, i) => `query-${i}`),
  ]);
  assert.deepEqual(readSearchHistory('owner-a').slice(0, 2), ['ZS', '财务 张']);
  assert.equal(readSearchHistory('owner-a').length, 12);
  assert.deepEqual(readSearchHistory('owner-b'), []);
  saveSearchHistory('owner-b', ['李四']);
  saveSearchHistory('owner-a', []);
  assert.deepEqual(readSearchHistory('owner-a'), []);
  assert.deepEqual(readSearchHistory('owner-b'), ['李四']);
  storage.set('langu-search-history:owner-a', 'invalid-json');
  assert.deepEqual(readSearchHistory('owner-a'), []);
  storage.set(
    'langu-search-history:owner-a',
    JSON.stringify([{}, null, 123, 'valid', 'a'.repeat(101)]),
  );
  assert.deepEqual(readSearchHistory('owner-a'), ['valid']);
  assert.deepEqual(saveSearchHistory('', ['unscoped']), []);
});

test('search keyboard leaves IME arrows and confirmation to the input method', async () => {
  let model;
  const app = createSSRApp({
    setup() {
      model = Dialog.setup({}, { expose() {}, emit() {} });
      return () => null;
    },
  });
  app.use(createPinia());
  await renderToString(app);
  const first = { id: '1', username: 'one' },
    second = { id: '2', username: 'two' };
  model.result.value.records = [first, second];
  model.selected.value = model.result.value.records[0];
  model.loading.value = false;
  for (const key of ['ArrowDown', 'ArrowUp', 'Enter']) {
    for (const mode of ['event', 'composition', 'webkit']) {
      model.composing.value = mode === 'composition';
      let prevented = false;
      model.onSearchKeydown({
        key,
        isComposing: mode === 'event',
        keyCode: mode === 'webkit' ? 229 : 0,
        preventDefault() {
          prevented = true;
        },
      });
      assert.equal(prevented, false);
      assert.equal(model.selected.value.id, '1');
      assert.equal(model.submitting.value, false);
    }
  }
  model.composing.value = false;
  let prevented = false;
  model.onSearchKeydown({
    key: 'ArrowDown',
    preventDefault() {
      prevented = true;
    },
  });
  await nextTick();
  assert.equal(prevented, true);
  assert.equal(model.selected.value.id, '2');
});

test('completed searches can be recalled and cleared; existing accounts use fast switching', async () => {
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, 'window');
  const commands = [];
  const user = { id: 'fixture-user', username: 'fixture.user' };
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      __TAURI_INTERNALS__: {
        invoke: async (command, args) => {
          commands.push({ command, args });
          if (command === 'search_users')
            return { records: [user], total: 1, page: 1, pageSize: 20 };
        },
      },
    },
  });
  let model;
  try {
    const app = createSSRApp({
      setup() {
        const store = useWorkspaceStore();
        store.state.directory.scope = 'integration-owner';
        store.state.sessions = [{ id: 'existing-view', account: user }];
        model = Dialog.setup({}, { expose() {}, emit() {} });
        return () => null;
      },
    });
    app.use(createPinia());
    await renderToString(app);
    model.keyword.value = ' 张三 ';
    await model.search();
    model.rememberSearch();
    assert.deepEqual(historyApi.readSearchHistory('integration-owner'), ['张三']);
    model.useHistory('张三');
    assert.equal(model.keyword.value, '张三');
    model.removeHistory('张三');
    model.rememberSearch();
    assert.deepEqual(historyApi.readSearchHistory('integration-owner'), []);
    await model.add();
    assert.deepEqual(commands.at(-1), { command: 'select_session', args: { id: 'existing-view' } });
    assert.ok(!commands.some(({ command }) => command === 'add_account'));
  } finally {
    model?.rememberSearch();
    if (previousWindow) Object.defineProperty(globalThis, 'window', previousWindow);
    else delete globalThis.window;
  }
});
