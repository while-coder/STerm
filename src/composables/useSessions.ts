// SSH 会话标签页：开/关/切换标签、SFTP 绑定、cd 跟随路径解析。模块级单例。
import { computed, ref, watch } from "vue";
import type { ConnectOpts, SavedConnection } from "../api";
import { useSettings } from "./useSettings";

export type SessionTab = {
  id: string;
  title: string;
  opts: Omit<ConnectOpts, "cols" | "rows">;
  remember: boolean;
  sftpHome: string;
  sftpCwd: string;
  sftpFollowPath: string;
  sftpFollowToken: number;
  connected: boolean;
  error: string;
};

const { settings } = useSettings();

const tabs = ref<SessionTab[]>([]);
const activeTabId = ref("");
const sftpTabId = ref("");

/** 当前 SFTP 面板绑定的会话（跟随模式跟随当前活动 tab）。 */
const activeSftpTab = computed<SessionTab | null>(() => {
  const targetId = settings.sftpFollowSsh ? activeTabId.value : sftpTabId.value;
  return (
    tabs.value.find((tab) => tab.id === targetId) ??
    tabs.value.find((tab) => tab.id === activeTabId.value) ??
    tabs.value[0] ??
    null
  );
});

let watcherBound = false;

function bindWatchers() {
  if (watcherBound) return;
  watcherBound = true;
  watch(
    () => settings.sftpFollowSsh,
    (follow) => {
      if (follow) {
        sftpTabId.value = activeTabId.value;
      } else if (!tabs.value.some((tab) => tab.id === sftpTabId.value)) {
        sftpTabId.value = activeTabId.value;
      }
    }
  );
}

function openSessionTab(conn: SavedConnection): string {
  const opts: Omit<ConnectOpts, "cols" | "rows"> = {
    id: crypto.randomUUID(),
    host: conn.host,
    port: Number(conn.port),
    username: conn.username,
    auth: conn.auth,
    password: conn.auth === "password" ? conn.password : undefined,
    privateKeyPath: conn.auth === "key" ? conn.privateKeyPath : undefined,
    passphrase: conn.auth === "key" ? conn.passphrase : undefined,
  };
  tabs.value.push({
    id: opts.id,
    title: conn.label || `${conn.username}@${conn.host}`,
    opts,
    remember: conn.remember,
    sftpHome: "/",
    sftpCwd: "/",
    sftpFollowPath: "",
    sftpFollowToken: 0,
    connected: false,
    error: "",
  });
  activeTabId.value = opts.id;
  if (settings.sftpFollowSsh || !sftpTabId.value) {
    sftpTabId.value = opts.id;
  }
  return opts.id;
}

function activateTab(id: string) {
  activeTabId.value = id;
  if (settings.sftpFollowSsh) {
    sftpTabId.value = id;
  }
}

function selectSftpTab(id: string) {
  sftpTabId.value = id;
}

function closeTab(id: string) {
  const index = tabs.value.findIndex((tab) => tab.id === id);
  if (index < 0) return;
  tabs.value.splice(index, 1);
  if (activeTabId.value === id) {
    const next = tabs.value[index] ?? tabs.value[index - 1];
    activeTabId.value = next?.id ?? "";
  }
  if (sftpTabId.value === id) {
    sftpTabId.value = activeTabId.value || tabs.value[0]?.id || "";
  }
  if (!tabs.value.length) {
    sftpTabId.value = "";
  }
}

function onTabConnected(tab: SessionTab) {
  tab.connected = true;
  tab.error = "";
}

// 终端经 OSC 7 上报的绝对工作目录，开启跟随时驱动 SFTP 面板定位。
function onTabCwd(tab: SessionTab, path: string) {
  if (!settings.sftpFollowSsh || !path) return;
  tab.sftpFollowPath = path;
  tab.sftpFollowToken += 1;
}

function onTabSftpCwdChanged(tab: SessionTab, path: string) {
  tab.sftpCwd = path;
}

function onTabSftpHomeChanged(tab: SessionTab, path: string) {
  tab.sftpHome = path;
}

export function useSessions() {
  bindWatchers();
  return {
    tabs,
    activeTabId,
    sftpTabId,
    activeSftpTab,
    openSessionTab,
    activateTab,
    selectSftpTab,
    closeTab,
    onTabConnected,
    onTabCwd,
    onTabSftpCwdChanged,
    onTabSftpHomeChanged,
  };
}
