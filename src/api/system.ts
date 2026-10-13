import { invokeCommand, isTauriRuntime } from '@/services/tauri';
import type { AppInfo } from '@/types/app';

export function getAppInfo() {
  if (!isTauriRuntime()) {
    return Promise.resolve<AppInfo>({
      name: '兰谷多账号工具',
      version: '0.1.0',
      runtime: 'Browser Preview',
    });
  }

  return invokeCommand<AppInfo>('get_app_info');
}
