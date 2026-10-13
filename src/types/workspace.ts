export interface Settings {
  loginUrl: string;
  apiBaseUrl: string;
  landingPath: string;
  remember: boolean;
  username: string;
}
export interface Account {
  id: string;
  username: string;
  avatar: string;
  realname: string;
  department: string;
}
export interface Session {
  id: string;
  role: 'primary' | 'secondary';
  account: Account | null;
  url: string;
  title: string;
  loading: boolean;
  status: 'pending' | 'checking' | 'ready' | 'error' | 'expired' | 'denied';
  zoom: number;
}
export interface WorkspaceSnapshot {
  settings: Settings;
  sessions: Session[];
  active: string | null;
  access: {
    status: 'pending' | 'checking' | 'ready' | 'denied' | 'expired' | 'unavailable';
    message: string;
  };
}
export interface UserPage {
  records: Account[];
  total: number;
  page: number;
  pageSize: number;
}
export interface BrowserBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
