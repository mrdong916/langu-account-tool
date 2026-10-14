import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import {
  desktopCommand,
  defaultSettings,
  emptyDirectory,
  initializeWorkspace,
  saveSettings,
} from '@/api/workspace';
import type { Settings, WorkspaceSnapshot } from '@/types/workspace';
import { isTauriRuntime } from '@/services/tauri';

export const useWorkspaceStore = defineStore('workspace', () => {
  const state = ref<WorkspaceSnapshot>({
    directory: { ...emptyDirectory },
    settings: { ...defaultSettings },
    sessions: [],
    active: null,
    access: { status: 'pending', message: '' },
  });
  const busy = ref(false);
  const error = ref('');
  const notice = ref('');
  const active = computed(() => state.value.sessions.find((s) => s.id === state.value.active));
  const primary = computed(() => state.value.sessions.find((s) => s.role === 'primary'));
  const secondary = computed(() => state.value.sessions.filter((s) => s.role === 'secondary'));
  const authorized = computed(() => state.value.access.status === 'ready');
  const apply = (snapshot: WorkspaceSnapshot) => {
    state.value = { ...snapshot, directory: snapshot.directory ?? { ...emptyDirectory } };
  };
  const report = (reason: unknown) => {
    error.value = reason instanceof Error ? reason.message : String(reason);
  };
  async function run(action: () => Promise<void>): Promise<boolean> {
    if (busy.value) return false;
    busy.value = true;
    error.value = '';
    notice.value = '';
    try {
      await action();
      return true;
    } catch (reason) {
      report(reason);
      return false;
    } finally {
      busy.value = false;
    }
  }
  async function initialize() {
    await run(async () => {
      apply(await initializeWorkspace());
      if (
        isTauriRuntime() &&
        state.value.settings.remember &&
        state.value.settings.username &&
        !primary.value
      ) {
        await desktopCommand('open_primary');
      }
    });
  }
  async function save(config: Settings, open = false) {
    return run(async () => {
      state.value.settings = await saveSettings(config);
      if (open) await desktopCommand('open_primary');
      else notice.value = '设置已保存';
    });
  }
  async function select(id: string) {
    await run(() => desktopCommand('select_session', { id }));
  }
  async function refreshDirectory() {
    if (state.value.directory.syncing) return;
    await run(() => desktopCommand('refresh_directory'));
  }
  async function browser(action: string, value?: string) {
    if (!active.value) return;
    await run(() => desktopCommand('browser_action', { id: active.value!.id, action, value }));
  }
  return {
    state,
    busy,
    error,
    notice,
    active,
    primary,
    secondary,
    authorized,
    apply,
    report,
    run,
    initialize,
    save,
    select,
    refreshDirectory,
    browser,
  };
});
