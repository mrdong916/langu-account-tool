<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { Search, X, ChevronLeft, ChevronRight, Check, UserPlus, RefreshCw } from 'lucide-vue-next';
import { searchUsers, desktopCommand } from '@/api/workspace';
import { useWorkspaceStore } from '@/stores/workspace';
import type { Account, UserPage } from '@/types/workspace';
import AccountAvatar from './AccountAvatar.vue';
const emit = defineEmits<{ close: [] }>();
const store = useWorkspaceStore();
const dialog = ref<HTMLDialogElement>();
const keyword = ref('');
const page = ref(1);
const result = ref<UserPage>({ records: [], total: 0, page: 1, pageSize: 20 });
const selected = ref<Account>();
const loading = ref(false);
const submitting = ref(false);
const error = ref('');
let generation = 0;
const pages = computed(() => Math.max(1, Math.ceil(result.value.total / 20)));
const existing = (user: Account) =>
  store.state.sessions.some((s) => s.account?.username === user.username);
async function search(nextPage = 1) {
  const current = ++generation;
  loading.value = true;
  error.value = '';
  selected.value = undefined;
  try {
    const response = await searchUsers(keyword.value, nextPage);
    if (current === generation) {
      result.value = response;
      page.value = nextPage;
    }
  } catch (reason) {
    if (current === generation) {
      error.value = String(reason);
      result.value.records = [];
    }
  } finally {
    if (current === generation) loading.value = false;
  }
}
async function add() {
  if (!selected.value || submitting.value) return;
  submitting.value = true;
  error.value = '';
  try {
    await desktopCommand('add_account', { username: selected.value.username });
    emit('close');
  } catch (reason) {
    error.value = String(reason);
  } finally {
    submitting.value = false;
  }
}
onMounted(() => {
  dialog.value?.showModal();
  void search();
});
onBeforeUnmount(() => {
  generation++;
});
</script>

<template>
  <dialog
    ref="dialog"
    class="account-dialog"
    aria-labelledby="search-title"
    @cancel.prevent="!submitting && emit('close')"
  >
    <header class="dialog-header">
      <div>
        <h2 id="search-title">添加子账号</h2>
        <p>主账号：{{ store.primary?.account?.username }}</p>
      </div>
      <button
        class="icon-button"
        title="关闭"
        aria-label="关闭"
        :disabled="submitting"
        @click="emit('close')"
      >
        <X :size="20" />
      </button>
    </header>
    <form class="user-search" @submit.prevent="search()">
      <div class="input-icon">
        <Search :size="18" /><input
          v-model="keyword"
          autofocus
          aria-label="检索用户名或工号"
          placeholder="检索用户名或工号"
          :disabled="submitting"
          maxlength="100"
        />
      </div>
      <button type="submit" class="button secondary" :disabled="loading || submitting">检索</button>
    </form>
    <div v-if="error" class="inline-error" role="alert">
      {{ error
      }}<button
        v-if="!submitting"
        class="icon-button"
        title="重新检索"
        aria-label="重新检索"
        @click="search(page)"
      >
        <RefreshCw :size="16" />
      </button>
    </div>
    <div class="user-results" :aria-busy="loading">
      <div v-if="loading" class="empty-state compact">正在检索用户…</div>
      <div v-else-if="!result.records.length" class="empty-state compact">
        {{ error ? '暂时无法加载用户' : '没有找到匹配的用户' }}
      </div>
      <template v-else>
        <button
          v-for="user in result.records"
          :key="user.id"
          type="button"
          class="user-result"
          :class="{ selected: selected?.id === user.id }"
          :disabled="submitting"
          :aria-pressed="selected?.id === user.id"
          @click="selected = user"
        >
          <AccountAvatar :src="user.avatar" :username="user.username" />
          <span class="user-description"
            ><strong>{{ user.username }}</strong
            ><small>{{ user.department || user.realname || '系统用户' }}</small></span
          >
          <span v-if="existing(user)" class="badge neutral">已打开</span
          ><Check v-if="selected?.id === user.id" :size="18" class="selection-check" />
        </button>
      </template>
    </div>
    <div class="pagination">
      <span>共 {{ result.total }} 个用户</span>
      <div>
        <button
          class="icon-button"
          title="上一页"
          aria-label="上一页"
          :disabled="page <= 1 || loading || submitting"
          @click="search(page - 1)"
        >
          <ChevronLeft :size="17" /></button
        ><span>{{ page }} / {{ pages }}</span
        ><button
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
    <footer class="dialog-footer">
      <span class="selected-name">{{ selected?.username || '未选择账号' }}</span
      ><button class="button primary" :disabled="!selected || submitting || loading" @click="add">
        <UserPlus :size="17" />{{
          submitting ? '正在登录…' : selected && existing(selected) ? '切换到账号' : '确认并打开'
        }}
      </button>
    </footer>
  </dialog>
</template>
