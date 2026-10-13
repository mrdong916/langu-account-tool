import { invokeCommand, isTauriRuntime } from '@/services/tauri';
import type { BrowserBounds, Settings, UserPage, WorkspaceSnapshot } from '@/types/workspace';

export const defaultSettings: Settings = {
  loginUrl: 'http://languwin.com/',
  apiBaseUrl: 'http://api.languwin.com',
  landingPath: '/',
  remember: true,
  username: '',
};

export async function initializeWorkspace(): Promise<WorkspaceSnapshot> {
  if (isTauriRuntime()) return invokeCommand('workspace_init');
  const saved = localStorage.getItem('langu-preview-settings');
  return {
    settings: saved ? { ...defaultSettings, ...JSON.parse(saved) } : { ...defaultSettings },
    sessions: [],
    active: null,
    access: { status: 'pending', message: '' },
  };
}
export async function saveSettings(config: Settings): Promise<Settings> {
  if (isTauriRuntime()) return invokeCommand('save_settings', { config });
  localStorage.setItem(
    'langu-preview-settings',
    JSON.stringify(config.remember ? config : { ...defaultSettings, remember: false }),
  );
  return { ...config };
}
export function desktopCommand<T = void>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauriRuntime())
    return Promise.reject(new Error('当前为浏览器预览，请在桌面客户端中打开账号会话。'));
  return invokeCommand<T>(command, args);
}
export const searchUsers = (keyword: string, page: number) =>
  desktopCommand<UserPage>('search_users', { keyword, page });
export const resizeBrowser = (bounds: BrowserBounds, visible: boolean) =>
  desktopCommand('browser_bounds', { bounds, visible });
