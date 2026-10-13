import { defineStore } from 'pinia';

import { getAppInfo } from '@/api/system';
import type { AppInfo } from '@/types/app';

interface AppState {
  appInfo: AppInfo | null;
  loading: boolean;
}

export const useAppStore = defineStore('app', {
  state: (): AppState => ({
    appInfo: null,
    loading: false,
  }),
  actions: {
    async loadAppInfo() {
      this.loading = true;
      try {
        this.appInfo = await getAppInfo();
      } finally {
        this.loading = false;
      }
    },
  },
});
