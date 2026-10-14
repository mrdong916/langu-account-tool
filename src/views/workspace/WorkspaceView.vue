<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  ArrowLeft,
  ArrowRight,
  Globe2,
  Plus,
  RefreshCw,
  Settings2,
  ShieldCheck,
  X,
  Wrench,
  ChevronLeft,
  ChevronRight,
  LogIn,
  ShieldAlert,
  Home,
  LogOut,
  Search,
  Minus,
  Users,
  Database,
} from 'lucide-vue-next';
import AccountAvatar from '@/components/account/AccountAvatar.vue';
import UserSearchDialog from '@/components/account/UserSearchDialog.vue';
import SettingsView from '@/views/settings/SettingsView.vue';
import { useWorkspaceStore } from '@/stores/workspace';
import { useNativeBrowser } from '@/hooks/useNativeBrowser';
import { desktopCommand, resizeBrowser } from '@/api/workspace';
import { isTauriRuntime } from '@/services/tauri';
import type { WorkspaceSnapshot } from '@/types/workspace';

const store = useWorkspaceStore();
const browserHost = ref<HTMLElement>();
const searchOpen = ref(false);
const closeDialog = ref<HTMLDialogElement>();
const confirmingClose = ref(false);
const settingsDialog = ref<HTMLDialogElement>();
const settingsOpen = ref(false);
const clearLogin = ref(false);
const address = ref('');
const addressFocused = ref(false);
const filter = ref('');
const collapsed = ref(localStorage.getItem('langu-sidebar-collapsed') === 'true');
const isMac = ref(false);
const primaryName = computed(() => {
  const account = store.primary?.account;
  return account?.realname && account.realname !== account.username ? account.realname : '主账号';
});
const gateBlocked = computed(() =>
  ['checking', 'denied', 'unavailable'].includes(store.state.access.status),
);
const visible = computed(
  () =>
    !!store.active &&
    !gateBlocked.value &&
    !searchOpen.value &&
    !confirmingClose.value &&
    !settingsOpen.value,
);
watch(collapsed, (value) => {
  localStorage.setItem('langu-sidebar-collapsed', String(value));
  filter.value = '';
});
watch(
  () => store.state.access.status,
  (status) => {
    if (status !== 'ready') searchOpen.value = false;
  },
);
function primaryStatus() {
  const status = store.state.access.status;
  return {
    pending: '等待登录',
    checking: '正在校验权限',
    ready: '已登录',
    denied: '暂无权限',
    expired: '登录已过期',
    unavailable: '校验失败',
  }[status];
}
const filtered = computed(() =>
  store.secondary.filter((s) =>
    filter.value
      .toLowerCase()
      .trim()
      .split(/\s+/)
      .every((term) =>
        (
          s.account?.searchText ||
          `${s.account?.username} ${s.account?.realname} ${s.account?.department}`
        )
          .toLowerCase()
          .includes(term),
      ),
  ),
);
const { update } = useNativeBrowser(browserHost, visible, store.report);
let unlisten: UnlistenFn | undefined;
let unlistenMenu: UnlistenFn | undefined;
let disposed = false;
watch(
  () => store.active?.url,
  (url) => {
    if (!addressFocused.value) address.value = url ?? '';
  },
);
watch(
  () => store.state.active,
  () => {
    address.value = store.active?.url ?? '';
    update();
  },
);
onMounted(async () => {
  isMac.value = /Mac/.test(navigator.platform);
  if (isTauriRuntime()) {
    unlisten = await listen<WorkspaceSnapshot>('workspace-changed', (event) =>
      store.apply(event.payload),
    );
    if (disposed) {
      unlisten();
      return;
    }
    unlistenMenu = await listen<string>(
      'desktop-action',
      (event) => void menuAction(event.payload),
    );
    if (disposed) {
      unlistenMenu();
      return;
    }
  }
  await store.initialize();
});
onBeforeUnmount(() => {
  disposed = true;
  unlisten?.();
  unlistenMenu?.();
});
async function menuAction(action: string) {
  if (store.busy || searchOpen.value || settingsOpen.value || confirmingClose.value) return;
  if (action === 'search') {
    if (store.primary) await openSearch();
  } else if (action === 'settings') await openSettings();
  else if (action === 'refresh-directory') await store.refreshDirectory();
  else if (action === 'close-workspace') {
    if (store.primary) await askClose();
  } else if (action === 'primary') {
    if (store.primary) await store.select('primary');
  } else if (action === 'previous' || action === 'next') {
    const sessions = store.state.sessions;
    if (!sessions.length) return;
    const index = sessions.findIndex((session) => session.id === store.state.active);
    const next = (index + (action === 'next' ? 1 : -1) + sessions.length) % sessions.length;
    await store.select(sessions[next].id);
  }
}
function startDrag(event: MouseEvent) {
  if (event.button === 0 && isTauriRuntime())
    void desktopCommand('drag_window').catch(store.report);
}
async function hideNative() {
  if (isTauriRuntime()) await resizeBrowser({ x: 0, y: 0, width: 0, height: 0 }, false);
}
async function openSearch() {
  if (!(await store.run(() => desktopCommand('check_primary_access')))) return;
  try {
    await hideNative();
    searchOpen.value = true;
  } catch (reason) {
    store.report(reason);
  }
}
async function askClose() {
  try {
    await hideNative();
    confirmingClose.value = true;
    clearLogin.value = false;
    closeDialog.value?.showModal();
  } catch (reason) {
    store.report(reason);
  }
}
async function openSettings() {
  if (!store.primary) return;
  try {
    await hideNative();
    settingsOpen.value = true;
    settingsDialog.value?.showModal();
  } catch (reason) {
    store.report(reason);
  }
}
function closeSettings() {
  settingsDialog.value?.close();
  settingsOpen.value = false;
}
async function editSettings() {
  closeSettings();
  await askClose();
}
function cancelClose() {
  closeDialog.value?.close();
  confirmingClose.value = false;
}
async function closeWorkspace() {
  if (await store.run(() => desktopCommand('close_workspace', { clear: clearLogin.value })))
    cancelClose();
}
</script>

<template>
  <div class="workspace-shell" :class="{ 'sidebar-collapsed': collapsed, 'is-mac': isMac }">
    <header class="native-titlebar" @mousedown="startDrag">
      <span>兰谷多账号工具</span><small>多账号工作区</small>
    </header>
    <nav class="app-rail" aria-label="功能导航">
      <button
        class="rail-button active"
        title="账号工作区"
        aria-label="账号工作区"
        @click="collapsed = !collapsed"
      >
        <Users :size="21" />
      </button>
      <button
        class="rail-button"
        title="搜索 / 添加账号（⌘/Ctrl+Shift+K）"
        aria-label="搜索或添加账号"
        :disabled="!store.primary || store.busy"
        @click="openSearch"
      >
        <Search :size="21" />
      </button>
      <button
        class="rail-button"
        :title="
          store.state.directory.error || `刷新用户缓存 · 已缓存 ${store.state.directory.count} 人`
        "
        aria-label="刷新用户缓存"
        :disabled="!store.authorized || store.state.directory.syncing || store.busy"
        @click="store.refreshDirectory"
      >
        <Database :size="20" :class="{ spinning: store.state.directory.syncing }" />
      </button>
      <span class="rail-spacer" />
      <button
        class="rail-button"
        title="系统设置"
        aria-label="系统设置"
        :disabled="store.busy"
        @click="openSettings"
      >
        <Settings2 :size="20" />
      </button>
      <button
        class="rail-button"
        title="关闭工作区"
        aria-label="关闭工作区"
        :disabled="!store.primary || store.busy"
        @click="askClose"
      >
        <LogOut :size="20" />
      </button>
    </nav>
    <aside class="account-sidebar">
      <div class="sidebar-header">
        <button
          class="account-item primary-profile"
          :class="{ active: store.state.active === 'primary' }"
          :disabled="store.busy || !store.primary"
          :title="`${primaryName} · ${primaryStatus()}`"
          :aria-label="`主账号，${primaryStatus()}`"
          :aria-pressed="store.state.active === 'primary'"
          @click="store.select('primary')"
        >
          <AccountAvatar :src="store.primary?.account?.avatar" primary />
          <span class="account-text">
            <strong>{{ primaryName }}</strong>
            <small>
              <span class="status-dot" :class="{ pending: !store.authorized }" />
              {{ primaryStatus() }}
            </small>
          </span>
        </button>
        <button
          class="icon-button sidebar-toggle"
          :title="collapsed ? '展开侧栏' : '折叠侧栏'"
          :aria-label="collapsed ? '展开侧栏' : '折叠侧栏'"
          :aria-expanded="!collapsed"
          @click="collapsed = !collapsed"
        >
          <ChevronRight v-if="collapsed" :size="16" /><ChevronLeft v-else :size="16" />
        </button>
      </div>
      <div class="sidebar-divider" />
      <div class="sidebar-label">
        子账号 <span class="count">{{ store.secondary.length }}</span
        ><button
          class="icon-button small"
          title="添加子账号"
          aria-label="添加子账号"
          :disabled="!store.primary || store.busy"
          @click="openSearch"
        >
          <Plus :size="17" />
        </button>
      </div>
      <div v-if="store.secondary.length" class="sidebar-filter input-icon">
        <Search :size="15" /><input
          v-model="filter"
          aria-label="筛选已打开的账号"
          placeholder="筛选已打开的账号"
        />
      </div>
      <div class="account-list">
        <div
          v-for="session in filtered"
          :key="session.id"
          class="account-row"
          :class="{
            active: store.state.active === session.id,
            expired: session.status === 'expired' || session.status === 'error',
          }"
        >
          <button
            class="account-item"
            :disabled="store.busy || !store.authorized"
            :title="session.account?.username"
            @click="store.select(session.id)"
          >
            <AccountAvatar
              :src="session.account?.avatar"
              :username="session.account?.username"
            /><span class="account-text"
              ><strong>{{ session.account?.realname || session.account?.username }}</strong
              ><small>{{
                session.status === 'ready'
                  ? session.account?.department || '子账号'
                  : session.status === 'error'
                    ? '登录失败'
                    : session.status === 'checking'
                      ? '正在登录'
                      : '登录已过期'
              }}</small></span
            >
          </button>
          <button
            class="icon-button small relogin-account"
            :title="`重新登录 ${session.account?.username}`"
            :aria-label="`重新登录 ${session.account?.username}`"
            :disabled="store.busy || !store.authorized || session.status === 'checking'"
            @click="store.run(() => desktopCommand('relogin_session', { id: session.id }))"
          >
            <LogIn :size="15" />
          </button>
          <button
            class="icon-button small close-account"
            :title="`关闭 ${session.account?.username}`"
            :aria-label="`关闭 ${session.account?.username}`"
            :disabled="store.busy"
            @click="store.run(() => desktopCommand('close_session', { id: session.id }))"
          >
            <X :size="14" />
          </button>
        </div>
        <div v-if="!store.secondary.length" class="sidebar-empty">暂无子账号</div>
        <div v-else-if="!filtered.length" class="sidebar-empty">没有匹配的账号</div>
      </div>
    </aside>
    <main class="workspace-main">
      <div v-if="store.state.access.status === 'expired'" class="message error" role="alert">
        主账号登录已过期，请重新登录
      </div>
      <div v-if="store.error" class="message error" role="alert">
        <span>{{ store.error }}</span
        ><button
          class="icon-button small"
          title="关闭提示"
          aria-label="关闭提示"
          @click="store.error = ''"
        >
          <X :size="16" />
        </button>
      </div>
      <div v-if="store.notice" class="message success" role="status">
        <span>{{ store.notice }}</span
        ><button
          class="icon-button small"
          title="关闭提示"
          aria-label="关闭提示"
          @click="store.notice = ''"
        >
          <X :size="16" />
        </button>
      </div>
      <section v-if="store.active && gateBlocked" class="access-gate" role="status">
        <ShieldAlert :size="32" />
        <h2>{{ store.state.access.message }}</h2>
        <div v-if="store.state.access.status !== 'checking'" class="form-actions">
          <button
            class="button secondary"
            :disabled="store.busy"
            @click="store.run(() => desktopCommand('check_primary_access'))"
          >
            <RefreshCw :size="16" />重新校验
          </button>
          <button
            class="button primary"
            :disabled="store.busy"
            @click="store.run(() => desktopCommand('relogin_primary'))"
          >
            <LogIn :size="16" />重新登录主账号
          </button>
        </div>
      </section>
      <template v-else-if="store.active">
        <form class="browser-toolbar" @submit.prevent="store.browser('navigate', address)">
          <div class="browser-navigation">
            <button
              type="button"
              class="icon-button"
              title="后退"
              aria-label="后退"
              :disabled="store.busy"
              @click="store.browser('back')"
            >
              <ArrowLeft :size="18" /></button
            ><button
              type="button"
              class="icon-button"
              title="前进"
              aria-label="前进"
              :disabled="store.busy"
              @click="store.browser('forward')"
            >
              <ArrowRight :size="18" /></button
            ><button
              type="button"
              class="icon-button"
              :title="store.active.loading ? '停止加载' : '刷新页面'"
              :aria-label="store.active.loading ? '停止加载' : '刷新页面'"
              :disabled="store.busy"
              @click="store.browser(store.active.loading ? 'stop' : 'reload')"
            >
              <X v-if="store.active.loading" :size="17" /><RefreshCw v-else :size="17" /></button
            ><button
              type="button"
              class="icon-button home-button"
              title="系统首页"
              aria-label="系统首页"
              :disabled="store.busy"
              @click="store.browser('home')"
            >
              <Home :size="17" />
            </button>
            <button
              type="button"
              class="icon-button"
              title="开发者工具"
              aria-label="开发者工具"
              :disabled="store.busy"
              @click="store.browser('devtools')"
            >
              <Wrench :size="18" />
            </button>
          </div>
          <div class="address-bar">
            <Globe2 :size="15" /><input
              v-model="address"
              aria-label="浏览器地址"
              @focus="addressFocused = true"
              @blur="addressFocused = false"
            /><button
              type="submit"
              class="icon-button small"
              title="打开地址"
              aria-label="打开地址"
              :disabled="store.busy"
            >
              <ArrowRight :size="16" />
            </button>
          </div>
          <div class="zoom-controls">
            <button
              type="button"
              class="icon-button small"
              title="缩小"
              aria-label="缩小"
              :disabled="store.busy || store.active.zoom <= 0.5"
              @click="store.browser('zoom', String(Math.max(0.5, store.active.zoom - 0.1)))"
            >
              <Minus :size="14" /></button
            ><output>{{ Math.round(store.active.zoom * 100) }}%</output
            ><button
              type="button"
              class="icon-button small"
              title="放大"
              aria-label="放大"
              :disabled="store.busy || store.active.zoom >= 2"
              @click="store.browser('zoom', String(Math.min(2, store.active.zoom + 0.1)))"
            >
              <Plus :size="14" />
            </button>
          </div>
        </form>
        <div ref="browserHost" class="browser-host">
          <div class="empty-state">
            <Globe2 :size="34" /><span>{{
              searchOpen || confirmingClose ? '账号页面已暂时隐藏' : '正在打开账号页面…'
            }}</span>
          </div>
        </div>
        <footer class="browser-status">
          <span
            ><ShieldCheck :size="13" />{{
              store.active.role === 'primary'
                ? `${primaryName} · ${primaryStatus()}`
                : store.active.account?.username
            }}</span
          ><span class="page-title">{{ store.active.title }}</span>
        </footer>
      </template>
      <SettingsView v-else />
    </main>
    <UserSearchDialog v-if="searchOpen" @close="searchOpen = false" />
    <dialog
      ref="settingsDialog"
      class="confirm-dialog settings-summary"
      aria-labelledby="settings-title"
      @cancel.prevent="closeSettings"
    >
      <h2 id="settings-title">系统设置</h2>
      <dl>
        <dt>系统登录地址</dt>
        <dd>{{ store.state.settings.loginUrl }}</dd>
        <dt>系统接口地址</dt>
        <dd>{{ store.state.settings.apiBaseUrl }}</dd>
        <dt>子账号登录后地址</dt>
        <dd>{{ store.state.settings.landingPath }}</dd>
        <dt>记住登录状态</dt>
        <dd>{{ store.state.settings.remember ? '已开启' : '已关闭' }}</dd>
      </dl>
      <div class="form-actions">
        <button class="button secondary" @click="closeSettings">返回</button>
        <button class="button primary" :disabled="store.busy" @click="editSettings">
          关闭工作区并修改
        </button>
      </div>
    </dialog>
    <dialog
      ref="closeDialog"
      class="confirm-dialog"
      aria-labelledby="close-title"
      @cancel.prevent="cancelClose"
    >
      <h2 id="close-title">关闭当前工作区？</h2>
      <p>主账号和所有子账号的浏览页面将关闭。</p>
      <label class="checkbox-row"
        ><input v-model="clearLogin" type="checkbox" />同时清除主账号的本地登录状态</label
      >
      <div class="form-actions">
        <button class="button secondary" :disabled="store.busy" @click="cancelClose">取消</button
        ><button class="button primary" :disabled="store.busy" @click="closeWorkspace">
          关闭工作区
        </button>
      </div>
    </dialog>
  </div>
</template>
