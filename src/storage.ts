// 已保存连接的本地持久化（localStorage）。机器列表整体加密保存；
// 密码仅在条目 remember 为 true 时进入加密载荷。
import type { SavedConnection } from "./api";
import { decryptJson, encryptJson, type EncryptedEnvelope } from "./crypto";

export type ThemeMode = "system" | "dark" | "light";

export interface AppSettings {
  theme: ThemeMode;
  showSftp: boolean;
  sftpFollowSsh: boolean;
  sftpAutoHome: boolean;
  /** 侧边栏折叠（仅桌面布局生效）。 */
  sidebarCollapsed: boolean;
  /** 侧边栏宽度（桌面，px）。 */
  sidebarWidth: number;
  /** SFTP 面板宽度（桌面，px）。 */
  sftpPanelWidth: number;
  /** 传输列表面板高度（px）。 */
  transferListHeight: number;
  /** 传输列表面板是否展开。 */
  transferListOpen: boolean;
  /** 终端配色方案 key（见 terminalThemes，"auto" 跟随界面主题）。 */
  termColorScheme: string;
  /** 终端字号（px）。 */
  termFontSize: number;
  /** 终端字体（CSS font-family）。 */
  termFontFamily: string;
  /** 并行传输上限（同时进行的上传 / 下载数）。 */
  maxParallelTransfers: number;
  /** 双击查看时的本地缓存目录；空表示用应用数据目录下的默认缓存。 */
  sftpCacheDir: string;
}

const CONNECTIONS_KEY = "sterm.connections";
const SETTINGS_KEY = "sterm.settings";
const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  showSftp: true,
  sftpFollowSsh: true,
  sftpAutoHome: true,
  sidebarCollapsed: false,
  sidebarWidth: 264,
  sftpPanelWidth: 340,
  transferListHeight: 180,
  transferListOpen: false,
  termColorScheme: "auto",
  termFontSize: 14,
  termFontFamily: "Consolas, 'Courier New', monospace",
  maxParallelTransfers: 2,
  sftpCacheDir: "",
};

interface ConnectionsStorePayload {
  connections: SavedConnection[];
}

function isEncryptedStore(value: unknown): value is EncryptedEnvelope {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as Partial<EncryptedEnvelope>).app === "STerm" &&
    (value as Partial<EncryptedEnvelope>).type === "connections-store"
  );
}

export async function loadConnections(masterPassword: string): Promise<SavedConnection[]> {
  try {
    const raw = localStorage.getItem(CONNECTIONS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (isEncryptedStore(parsed)) {
      const payload = await decryptJson<ConnectionsStorePayload>(
        raw,
        masterPassword,
        "connections-store"
      );
      return Array.isArray(payload?.connections) ? payload.connections : [];
    }
    return [];
  } catch {
    throw new Error("机器列表读取失败：主密码错误或本地数据已损坏");
  }
}

export async function saveConnections(list: SavedConnection[], masterPassword: string): Promise<void> {
  if (!masterPassword) throw new Error("缺少主密码，无法保存机器列表");
  const payload: ConnectionsStorePayload = { connections: list };
  const text = await encryptJson(payload, masterPassword, "connections-store");
  localStorage.setItem(CONNECTIONS_KEY, text);
}

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(settings: AppSettings): void {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

// 按 id 插入或更新一条；不存密码时清掉敏感字段。返回新列表。
export function upsertConnection(
  list: SavedConnection[],
  conn: SavedConnection
): SavedConnection[] {
  const clean: SavedConnection = { ...conn };
  if (!clean.remember) {
    delete clean.password;
    delete clean.passphrase;
  }
  const idx = list.findIndex((c) => c.id === clean.id);
  return idx >= 0 ? list.map((c, i) => (i === idx ? clean : c)) : [...list, clean];
}

export function removeConnection(list: SavedConnection[], id: string): SavedConnection[] {
  return list.filter((c) => c.id !== id);
}
