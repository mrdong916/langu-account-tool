import test, { after, before } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { createSSRApp } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { createPinia } from 'pinia';

let server;
let WorkspaceView;
let useWorkspaceStore;
const previousStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');

before(async () => {
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: { getItem: () => null },
  });
  server = await createServer({ server: { middlewareMode: true, hmr: false, watch: null } });
  WorkspaceView = (await server.ssrLoadModule('/src/views/workspace/WorkspaceView.vue')).default;
  ({ useWorkspaceStore } = await server.ssrLoadModule('/src/stores/workspace.ts'));
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
