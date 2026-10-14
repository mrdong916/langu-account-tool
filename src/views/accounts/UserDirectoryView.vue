<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  Search,
  ArrowLeft,
  ChevronLeft,
  ChevronRight,
  ArrowUpRight,
  RefreshCw,
} from 'lucide-vue-next';
import { searchUsers, desktopCommand } from '@/api/workspace';
import { useWorkspaceStore } from '@/stores/workspace';
import type { Account, UserPage } from '@/types/workspace';
import AccountAvatar from '@/components/account/AccountAvatar.vue';
import SearchHighlight from '@/components/account/SearchHighlight.vue';
const props = defineProps<{ mode: 'search' | 'directory' }>();
const emit = defineEmits<{ close: []; opened: [] }>();
const store = useWorkspaceStore();
const list = ref<HTMLElement>();
const input = ref<HTMLInputElement>();
const keyword = ref('');
const page = ref(1);
const result = ref<UserPage>({ records: [], total: 0, page: 1, pageSize: 20 });
const selected = ref<Account>();
const loading = ref(false);
const submitting = ref(false);
const error = ref('');
const hasQuery = computed(() => props.mode === 'directory' || !!keyword.value.trim());
let generation = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
const pages = computed(() => Math.max(1, Math.ceil(result.value.total / result.value.pageSize)));
const existing = (user: Account) =>
  store.state.sessions.some((s) => s.account?.username === user.username);
const syncTime = computed(() =>
  store.state.directory.lastSync
    ? new Date(store.state.directory.lastSync).toLocaleString()
    : '尚未完成同步',
);
async function search(nextPage = 1) {
  clearTimeout(timer);
  const current = ++generation;
  loading.value = true;
  error.value = '';
  selected.value = undefined;
  if (!hasQuery.value) {
    result.value = { records: [], total: 0, page: 1, pageSize: 20 };
    page.value = 1;
    loading.value = false;
    return;
  }
  try {
    const response = await searchUsers(keyword.value, nextPage);
    if (current === generation) {
      result.value = response;
      page.value = response.page;
      selected.value = response.records[0];
    }
  } catch (reason) {
    if (current === generation) {
      error.value = String(reason);
      result.value = { records: [], total: 0, page: 1, pageSize: 20 };
    }
  } finally {
    if (current === generation) loading.value = false;
  }
}
watch(keyword, () => {
  generation++;
  selected.value = undefined;
  loading.value = true;
  clearTimeout(timer);
  timer = setTimeout(() => void search(), 120);
});
watch(
  () => store.state.directory.lastSync,
  () => void search(),
);
watch(
  () => props.mode,
  async () => {
    keyword.value = '';
    await search();
    input.value?.focus();
  },
);
async function moveSelection(direction: number) {
  if (loading.value || submitting.value) return;
  const index = result.value.records.findIndex((user) => user.id === selected.value?.id);
  selected.value =
    result.value.records[Math.max(0, Math.min(result.value.records.length - 1, index + direction))];
  await nextTick();
  list.value?.querySelector('.user-result.selected')?.scrollIntoView({ block: 'nearest' });
}
async function add(user = selected.value) {
  if (!user || submitting.value || loading.value || store.busy) return;
  selected.value = user;
  submitting.value = true;
  error.value = '';
  try {
    if (await store.run(() => desktopCommand('add_account', { username: user.username })))
      emit('opened');
  } finally {
    submitting.value = false;
  }
}
onMounted(() => {
  input.value?.focus();
  void search();
});
onBeforeUnmount(() => {
  generation++;
  clearTimeout(timer);
});
</script>

<template>
  <section class="directory-page" aria-labelledby="search-title">
    <header class="dialog-header">
      <div>
        <h2 id="search-title">{{ mode === 'directory' ? '用户列表' : '搜索账号' }}</h2>
        <p>仅显示正常状态账号 · 支持姓名、账号、工号、部门和拼音检索</p>
      </div>
      <button
        class="icon-button"
        title="返回账号页面"
        aria-label="返回账号页面"
        :disabled="submitting"
        @click="emit('close')"
      >
        <ArrowLeft :size="20" />
      </button>
    </header>
    <form class="user-search" @submit.prevent="add()">
      <div class="input-icon">
        <Search :size="18" /><input
          ref="input"
          v-model="keyword"
          aria-label="检索姓名、账号、工号、部门或拼音"
          placeholder="试试「张三」「zhangsan」「zs」或「财务 张」"
          :disabled="submitting"
          maxlength="100"
          @keydown.down.prevent="moveSelection(1)"
          @keydown.up.prevent="moveSelection(-1)"
        />
      </div>
    </form>
    <div class="directory-status" role="status">
      <span v-if="store.state.directory.syncing"
        ><RefreshCw :size="13" class="spinning" />正在同步 {{ store.state.directory.fetched }} /
        {{ store.state.directory.total || '…' }}</span
      >
      <span v-else :title="`上次同步：${syncTime}`"
        >已缓存 {{ store.state.directory.count }} 人 · {{ syncTime }}</span
      >
      <button
        class="text-button"
        :disabled="store.state.directory.syncing || store.busy || submitting"
        @click="store.refreshDirectory"
      >
        刷新用户缓存
      </button>
    </div>
    <div v-if="store.state.directory.error" class="inline-error" role="alert">
      {{ store.state.directory.error }}；现有缓存仍可检索。
    </div>
    <div v-if="error" class="inline-error" role="alert">
      {{ error
      }}<button class="icon-button" aria-label="重试检索" @click="search(page)">
        <RefreshCw :size="16" />
      </button>
    </div>
    <div ref="list" class="user-results" :aria-busy="loading">
      <div v-if="loading" class="empty-state compact">正在检索用户…</div>
      <div v-else-if="!result.records.length" class="empty-state compact">
        {{
          !hasQuery
            ? '输入姓名、账号、工号、部门或拼音，即时查找账号'
            : store.state.directory.syncing && !store.state.directory.count
              ? '首次同步中，完成后即可检索'
              : '没有找到匹配的用户'
        }}
      </div>
      <template v-else>
        <div
          v-for="user in result.records"
          :key="user.id"
          class="user-result"
          :class="{ selected: selected?.id === user.id }"
          @click="selected = user"
        >
          <AccountAvatar :src="user.avatar" :username="user.username" />
          <span class="user-description">
            <strong
              ><SearchHighlight :text="user.realname || user.username" :query="keyword"
            /></strong>
            <small
              >账号：<SearchHighlight :text="user.username" :query="keyword" /><template
                v-if="user.workNo"
              >
                · 工号：<SearchHighlight :text="user.workNo" :query="keyword" /></template
            ></small>
            <small
              >部门：<SearchHighlight :text="user.department || '未提供部门'" :query="keyword"
            /></small>
          </span>
          <button
            class="button secondary open-account"
            :disabled="submitting || loading || store.busy"
            :aria-label="`${existing(user) ? '切换到' : '直接打开'} ${user.realname || user.username}`"
            @click.stop="add(user)"
          >
            <ArrowUpRight :size="16" />{{
              submitting && selected?.id === user.id
                ? '正在打开…'
                : existing(user)
                  ? '切换到账号'
                  : '直接打开'
            }}
          </button>
        </div>
      </template>
    </div>
    <div v-if="hasQuery" class="pagination">
      <span>找到 {{ result.total }} 个用户 · ↑↓ 选择，Enter 打开</span>
      <div>
        <button
          class="icon-button"
          title="上一页"
          aria-label="上一页"
          :disabled="page <= 1 || loading || submitting"
          @click="search(page - 1)"
        >
          <ChevronLeft :size="17" /></button
        ><span>{{ page }} / {{ pages }}</span>
        <button
          class="icon-button"
          title="下一页"
          aria-label="下一页"
          :disabled="page >= pages || loading || submitting"
          @click="search(page + 1)"
        >
          <ChevronRight :size="17" />
        </button>
      </div>
    </div>
  </section>
</template>
