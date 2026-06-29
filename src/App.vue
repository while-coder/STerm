<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import TerminalView from "./components/TerminalView.vue";
import FileBrowser from "./components/FileBrowser.vue";
import { defaultPrivateKeyPath, type AuthMethod, type ConnectOpts, type SavedConnection } from "./api";
import {
  loadConnections,
  loadSettings,
  removeConnection,
  saveSettings,
  upsertConnection,
  type AppSettings,
} from "./storage";

type Panel = "machines" | "form" | "settings" | null;
type ResolvedTheme = "dark" | "light";
type SessionTab = {
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

const panel = ref<Panel>("machines");
const tabs = ref<SessionTab[]>([]);
const activeTabId = ref("");
const sftpTabId = ref("");
const connectError = ref("");
const FALLBACK_PRIVATE_KEY_PATH = "~/.ssh/id_rsa";
const defaultKeyPath = ref(FALLBACK_PRIVATE_KEY_PATH);
const settings = reactive<AppSettings>(loadSettings());
const resolvedTheme = ref<ResolvedTheme>("dark");
let themeQuery: MediaQueryList | null = null;

function resolveTheme(): ResolvedTheme {
  if (settings.theme === "dark" || settings.theme === "light") return settings.theme;
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function applyTheme() {
  resolvedTheme.value = resolveTheme();
  document.documentElement.dataset.theme = resolvedTheme.value;
}

// 已保存的连接列表。
const saved = ref<SavedConnection[]>(loadConnections());
const activeSftpTab = computed(() => {
  const targetId = settings.sftpFollowSsh ? activeTabId.value : sftpTabId.value;
  return (
    tabs.value.find((tab) => tab.id === targetId) ??
    tabs.value.find((tab) => tab.id === activeTabId.value) ??
    tabs.value[0] ??
    null
  );
});

const form = reactive({
  host: "",
  port: 22,
  username: "",
  auth: "password" as AuthMethod,
  password: "",
  privateKeyPath: FALLBACK_PRIVATE_KEY_PATH,
  passphrase: "",
  remember: false,
});

onMounted(async () => {
  applyTheme();
  themeQuery = window.matchMedia("(prefers-color-scheme: light)");
  themeQuery.addEventListener("change", applyTheme);
  try {
    const previous = defaultKeyPath.value;
    const path = await defaultPrivateKeyPath();
    defaultKeyPath.value = path || FALLBACK_PRIVATE_KEY_PATH;
    if (!form.privateKeyPath || form.privateKeyPath === previous) {
      form.privateKeyPath = defaultKeyPath.value;
    }
  } catch {
    defaultKeyPath.value = FALLBACK_PRIVATE_KEY_PATH;
  }
});

onBeforeUnmount(() => {
  themeQuery?.removeEventListener("change", applyTheme);
});

watch(() => settings.theme, applyTheme);
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
watch(settings, () => saveSettings(settings), { deep: true });

async function pickKey() {
  const p = await open({ multiple: false });
  if (typeof p === "string") form.privateKeyPath = p;
}

function resetForm() {
  form.host = "";
  form.port = 22;
  form.username = "";
  form.auth = "password";
  form.password = "";
  form.privateKeyPath = defaultKeyPath.value;
  form.passphrase = "";
  form.remember = false;
}

function newConnection() {
  resetForm();
  connectError.value = "";
  panel.value = "form";
}

function backToMachines() {
  connectError.value = "";
  panel.value = "machines";
}

function cancelForm() {
  if (tabs.value.length) {
    closePanel();
  } else {
    backToMachines();
  }
}

function closePanel() {
  if (tabs.value.length) {
    connectError.value = "";
    panel.value = null;
  }
}

function currentSavedConnection(): SavedConnection | null {
  const host = form.host.trim();
  const username = form.username.trim();
  if (!host || !username) {
    connectError.value = "保存前请填写主机和用户名";
    return null;
  }
  connectError.value = "";
  const existing = saved.value.find(
    (c) =>
      c.host === host &&
      c.port === Number(form.port) &&
      c.username === username &&
      c.auth === form.auth
  );
  return {
    id: existing?.id ?? crypto.randomUUID(),
    label: `${username}@${host}`,
    host,
    port: Number(form.port),
    username,
    auth: form.auth,
    privateKeyPath: form.auth === "key" ? form.privateKeyPath : undefined,
    remember: form.remember,
    password: form.auth === "password" ? form.password : undefined,
    passphrase: form.auth === "key" ? form.passphrase || undefined : undefined,
  };
}

// 将当前表单保存为一条连接（按 host:port/username 判重，已存在则更新）。
function saveCurrent() {
  const conn = currentSavedConnection();
  if (!conn) return;
  saved.value = upsertConnection(saved.value, conn);
  panel.value = tabs.value.length ? null : "machines";
}

function saveAndConnect() {
  const conn = currentSavedConnection();
  if (!conn) return;
  saved.value = upsertConnection(saved.value, conn);
  openSessionTab(conn);
}

// 用某条已保存连接填充表单。
function applyConn(c: SavedConnection) {
  form.host = c.host;
  form.port = c.port;
  form.username = c.username;
  form.auth = c.auth;
  form.privateKeyPath = c.privateKeyPath ?? defaultKeyPath.value;
  form.remember = c.remember;
  form.password = c.remember ? c.password ?? "" : "";
  form.passphrase = c.remember ? c.passphrase ?? "" : "";
}

function deleteConn(c: SavedConnection) {
  saved.value = removeConnection(saved.value, c.id);
}

function editConnection(c: SavedConnection) {
  applyConn(c);
  connectError.value = "";
  panel.value = "form";
}

// 填充并直接连接（密码未保存时回到表单等待手动输入）。
function connectSaved(c: SavedConnection) {
  applyConn(c);
  if (c.auth === "password" && !c.password) {
    connectError.value = "请输入密码后连接";
    panel.value = "form";
    return;
  }
  openSessionTab(c);
}

function openSessionTab(conn: SavedConnection) {
  connectError.value = "";
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
  panel.value = null;
}

function showMachineList() {
  panel.value = "machines";
}

function showSettings() {
  connectError.value = "";
  panel.value = "settings";
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
    panel.value = "machines";
  }
}

function onTabConnected(tab: SessionTab) {
  tab.connected = true;
  tab.error = "";
  connectError.value = "";
}

function normalizeRemotePath(path: string): string {
  const isAbs = path.startsWith("/");
  const parts: string[] = [];
  for (const part of path.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      parts.pop();
    } else {
      parts.push(part);
    }
  }
  return `${isAbs ? "/" : ""}${parts.join("/")}` || "/";
}

function resolveRemotePath(tab: SessionTab, target: string): string {
  const base = tab.sftpCwd || tab.sftpHome || "/";
  const home = tab.sftpHome || base || "/";
  if (!target || target === "~") return home;
  if (target.startsWith("~/")) return normalizeRemotePath(`${home}/${target.slice(2)}`);
  if (target.startsWith("/")) return normalizeRemotePath(target);
  return normalizeRemotePath(`${base || "/"}/${target}`);
}

function onTabCwdCommand(tab: SessionTab, target: string) {
  if (!settings.sftpFollowSsh) return;
  tab.sftpFollowPath = resolveRemotePath(tab, target);
  tab.sftpFollowToken += 1;
}

function onTabSftpCwdChanged(tab: SessionTab, path: string) {
  tab.sftpCwd = path;
}

function onTabSftpHomeChanged(tab: SessionTab, path: string) {
  tab.sftpHome = path;
}

function onTabError(tab: SessionTab, msg: string) {
  tab.error = msg;
  connectError.value = msg;
  if (!tab.connected) {
    applyConn({
      id: "",
      label: tab.title,
      host: tab.opts.host,
      port: tab.opts.port,
      username: tab.opts.username,
      auth: tab.opts.auth,
      privateKeyPath: tab.opts.privateKeyPath,
      remember: tab.remember,
      password: tab.opts.password,
      passphrase: tab.opts.passphrase,
    });
    closeTab(tab.id);
    panel.value = "form";
  }
}
</script>

<template>
  <div class="app">
    <!-- 多 SSH 会话：所有 tab 保持挂载，切换时只隐藏非当前 tab。 -->
    <div v-if="tabs.length" class="workspace">
      <div class="tabbar">
        <div class="tabstrip">
          <div
            v-for="tab in tabs"
            :key="tab.id"
            class="tab"
            :class="{ active: tab.id === activeTabId }"
            :title="`${tab.title} - ${tab.opts.username}@${tab.opts.host}:${tab.opts.port}`"
            @click="activateTab(tab.id)"
          >
            <span class="tab-status" :class="{ connected: tab.connected, error: tab.error }"></span>
            <span class="tab-title">{{ tab.title }}</span>
            <button class="tab-close" type="button" title="关闭" @click.stop="closeTab(tab.id)">×</button>
          </div>
          <button class="tab-add" type="button" title="打开机器列表" @click="showMachineList">+</button>
        </div>
        <div class="tab-actions">
          <button type="button" @click="showSettings">设置</button>
        </div>
      </div>

      <div class="workspace-body">
        <div class="ssh-area">
          <section
            v-for="tab in tabs"
            :key="tab.id"
            v-show="tab.id === activeTabId"
            class="tab-session"
          >
            <div class="bar">
              <span>{{ tab.opts.username }}@{{ tab.opts.host }}</span>
              <span v-if="tab.error" class="bar-error">{{ tab.error }}</span>
              <button @click="closeTab(tab.id)">断开</button>
            </div>
            <div class="pane term-pane">
              <TerminalView
                :opts="tab.opts"
                :active="tab.id === activeTabId"
                :theme="resolvedTheme"
                @connected="onTabConnected(tab)"
                @error="onTabError(tab, $event)"
                @closed="closeTab(tab.id)"
                @cwd-command="onTabCwdCommand(tab, $event)"
              />
            </div>
          </section>
        </div>
        <div v-if="settings.showSftp && activeSftpTab" class="pane file-pane">
          <div v-if="!settings.sftpFollowSsh" class="sftp-bind">
            <span>SFTP</span>
            <select v-model="sftpTabId" @change="selectSftpTab(sftpTabId)">
              <option v-for="tab in tabs" :key="tab.id" :value="tab.id">
                {{ tab.title }} - {{ tab.opts.username }}@{{ tab.opts.host }}
              </option>
            </select>
          </div>
          <div class="sftp-browser">
            <FileBrowser
              :key="activeSftpTab.id"
              :id="activeSftpTab.id"
              :connected="activeSftpTab.connected"
              :auto-home="settings.sftpAutoHome"
              :follow-path="activeSftpTab.sftpFollowPath"
              :follow-token="activeSftpTab.sftpFollowToken"
              @cwd-changed="onTabSftpCwdChanged(activeSftpTab, $event)"
              @home-changed="onTabSftpHomeChanged(activeSftpTab, $event)"
            />
          </div>
        </div>
      </div>
    </div>

    <div
      v-if="panel === 'machines'"
      class="panel-shell"
      :class="{ overlay: tabs.length }"
      @click.self="closePanel"
    >
      <div class="machine-page">
        <header class="machine-header">
          <div>
            <h1>STerm</h1>
            <p class="sub">选择机器或新建 SSH / SFTP 连接</p>
          </div>
          <div class="header-actions">
            <button v-if="tabs.length" type="button" @click="closePanel">关闭</button>
            <button type="button" @click="showSettings">设置</button>
            <button class="primary" type="button" @click="newConnection">新建连接</button>
          </div>
        </header>

        <section class="machine-list">
          <div class="machine-list-title">机器列表</div>
          <ul v-if="saved.length">
            <li v-for="c in saved" :key="c.id" class="machine-item">
              <button class="machine-main" :title="`${c.label} (${c.host}:${c.port})`" @click="connectSaved(c)">
                <span class="name">{{ c.label }}</span>
                <span class="meta">{{ c.username }}@{{ c.host }}:{{ c.port }} · {{ c.auth === "key" ? "私钥" : "密码" }}</span>
              </button>
              <button class="secondary" type="button" @click="editConnection(c)">编辑</button>
              <button class="secondary" type="button" @click="connectSaved(c)">
                {{ c.auth === "password" && !c.password ? "填写密码" : "连接" }}
              </button>
              <button class="danger" type="button" @click="deleteConn(c)">删除</button>
            </li>
          </ul>
          <div v-else class="empty">
            <div class="empty-title">还没有保存机器</div>
            <p>新建一个连接后，它会出现在这里。</p>
            <button class="primary" type="button" @click="newConnection">新建连接</button>
          </div>
        </section>
      </div>
    </div>

    <div
      v-if="panel === 'form'"
      class="panel-shell"
      :class="{ overlay: tabs.length }"
      @click.self="closePanel"
    >
      <div class="form-wrap">
        <h1>连接配置</h1>
        <p class="sub">填写机器信息后可以直接连接，也可以保存到列表</p>

        <form class="form" @submit.prevent="saveAndConnect">
          <div class="row">
            <input v-model="form.host" placeholder="主机 (host)" required />
            <input v-model.number="form.port" type="number" placeholder="端口" class="port" />
          </div>
          <input v-model="form.username" placeholder="用户名" required />

          <div class="auth-tabs">
            <label><input type="radio" value="password" v-model="form.auth" /> 密码</label>
            <label><input type="radio" value="key" v-model="form.auth" /> 私钥</label>
          </div>

          <input
            v-if="form.auth === 'password'"
            v-model="form.password"
            type="password"
            placeholder="密码"
          />
          <template v-else>
            <div class="row">
              <input v-model="form.privateKeyPath" placeholder="私钥文件路径" />
              <button type="button" @click="pickKey">选择…</button>
            </div>
            <input v-model="form.passphrase" type="password" placeholder="私钥口令（可选）" />
          </template>

          <label class="remember">
            <input type="checkbox" v-model="form.remember" />
            记住密码/口令
          </label>

          <div class="actions">
            <button type="button" @click="cancelForm">{{ tabs.length ? "取消" : "返回列表" }}</button>
            <button type="button" @click="saveCurrent">保存到列表</button>
            <button type="submit" class="primary">保存并连接</button>
          </div>
          <p v-if="connectError" class="error">{{ connectError }}</p>
        </form>
      </div>
    </div>

    <div
      v-if="panel === 'settings'"
      class="panel-shell"
      :class="{ overlay: tabs.length }"
      @click.self="closePanel"
    >
      <div class="settings-page">
        <header class="machine-header">
          <div>
            <h1>设置</h1>
            <p class="sub">调整界面和 SFTP 行为</p>
          </div>
          <div class="header-actions">
            <button v-if="tabs.length" type="button" @click="closePanel">关闭</button>
            <button v-else type="button" @click="backToMachines">返回列表</button>
          </div>
        </header>

        <section class="settings-section">
          <div class="settings-title">主题</div>
          <div class="segmented">
            <label :class="{ active: settings.theme === 'system' }">
              <input v-model="settings.theme" type="radio" value="system" />
              跟随系统
            </label>
            <label :class="{ active: settings.theme === 'dark' }">
              <input v-model="settings.theme" type="radio" value="dark" />
              深色
            </label>
            <label :class="{ active: settings.theme === 'light' }">
              <input v-model="settings.theme" type="radio" value="light" />
              浅色
            </label>
          </div>
        </section>

        <section class="settings-section">
          <div class="settings-title">SFTP</div>
          <label class="setting-row">
            <span>
              <strong>显示 SFTP 文件面板</strong>
              <small>关闭后只显示 SSH 终端。</small>
            </span>
            <input v-model="settings.showSftp" type="checkbox" />
          </label>
          <label class="setting-row" :class="{ disabled: !settings.showSftp }">
            <span>
              <strong>SFTP 自动跟随 SSH</strong>
              <small>开启后切换 SSH 标签页时，文件面板自动切到当前会话。</small>
            </span>
            <input v-model="settings.sftpFollowSsh" type="checkbox" :disabled="!settings.showSftp" />
          </label>
          <label class="setting-row" :class="{ disabled: !settings.showSftp }">
            <span>
              <strong>连接后自动加载主目录</strong>
              <small>关闭后需要在 SFTP 面板里手动刷新。</small>
            </span>
            <input v-model="settings.sftpAutoHome" type="checkbox" :disabled="!settings.showSftp" />
          </label>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
:global(:root) {
  --app-bg: #1e1e1e;
  --surface: #252525;
  --surface-2: #2a2a2a;
  --surface-3: #333;
  --line: #444;
  --text: #eaeaea;
  --muted: rgba(234, 234, 234, 0.62);
  --terminal-bg: #1e1e1e;
}
:global(:root[data-theme="light"]) {
  --app-bg: #f5f6f8;
  --surface: #ffffff;
  --surface-2: #f1f3f5;
  --surface-3: #e9ecef;
  --line: #d0d7de;
  --text: #1f2328;
  --muted: rgba(31, 35, 40, 0.62);
  --terminal-bg: #ffffff;
}
.app {
  position: relative;
  height: 100vh;
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  background: var(--app-bg);
}
.workspace {
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-width: 0;
}
.tabbar {
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  gap: 8px;
  height: 38px;
  background: var(--surface-2);
  border-bottom: 1px solid var(--line);
}
.tabstrip {
  display: flex;
  align-items: stretch;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
}
.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 150px;
  max-width: 240px;
  padding: 0 8px;
  border-right: 1px solid var(--line);
  color: var(--muted);
  cursor: pointer;
  background: var(--surface-2);
}
.tab:hover {
  background: var(--surface-3);
}
.tab.active {
  background: var(--surface);
  color: var(--text);
}
.tab-status {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #777;
  flex: 0 0 auto;
}
.tab-status.connected {
  background: #2bd576;
}
.tab-status.error {
  background: #f48771;
}
.tab-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-weight: 600;
}
.tab-close {
  padding: 0 5px;
  border: 0;
  background: transparent;
  color: #aaa;
}
.tab-close:hover {
  color: var(--text);
  background: var(--surface-3);
}
.tab-add {
  min-width: 38px;
  height: 100%;
  padding: 0;
  border: 0;
  border-right: 1px solid var(--line);
  border-radius: 0;
  background: transparent;
  color: var(--muted);
  font-size: 20px;
  line-height: 1;
}
.tab-add:hover {
  color: var(--text);
  background: var(--surface-3);
}
.tab-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  flex: 0 0 auto;
}
.tab-actions button {
  height: 28px;
  padding: 0 10px;
}
.workspace-body {
  display: flex;
  flex: 1;
  min-height: 0;
}
.ssh-area {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.tab-session {
  display: flex;
  flex-direction: column;
  flex: 1;
  height: 100%;
  min-height: 0;
}
.panel-shell {
  height: 100vh;
  overflow: auto;
}
.panel-shell.overlay {
  position: absolute;
  inset: 38px 0 0;
  z-index: 10;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: 24px;
  background: rgba(0, 0, 0, 0.48);
}
.machine-page {
  max-width: 960px;
  margin: 0 auto;
  padding: 32px;
  min-height: 100vh;
}
.machine-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 28px;
}
.panel-shell.overlay .machine-page,
.panel-shell.overlay .form-wrap,
.panel-shell.overlay .settings-page {
  width: min(960px, 100%);
  max-height: calc(100vh - 86px);
  overflow: auto;
  padding: 24px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--app-bg);
  box-shadow: 0 18px 50px rgba(0, 0, 0, 0.32);
}
.panel-shell.overlay .machine-page {
  min-height: auto;
}
.panel-shell.overlay .form-wrap,
.panel-shell.overlay .settings-page {
  margin: 0;
}
.header-actions {
  display: flex;
  gap: 8px;
  flex: 0 0 auto;
}
.form-wrap {
  max-width: 380px;
  margin: 8vh auto;
  text-align: center;
}
h1 {
  margin: 0;
  font-size: 2.2rem;
}
.sub {
  color: var(--muted);
  margin: 4px 0 24px;
}
.machine-list {
  text-align: left;
}
.machine-list-title {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  margin-bottom: 8px;
}
.machine-list ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.machine-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto auto;
  gap: 8px;
  align-items: stretch;
  padding: 8px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.machine-main {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 3px;
  min-width: 0;
  text-align: left;
}
.machine-main .name {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.98rem;
}
.machine-main .meta {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.78rem;
  color: var(--muted);
}
.empty {
  padding: 28px;
  border: 1px dashed var(--line);
  border-radius: 8px;
  text-align: center;
  background: var(--surface);
}
.empty-title {
  font-size: 1rem;
  margin-bottom: 6px;
}
.empty p {
  margin: 0 0 16px;
  color: var(--muted);
}
.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  text-align: left;
}
.row {
  display: flex;
  gap: 8px;
}
.row input:first-child {
  flex: 1;
}
.port {
  width: 90px;
}
.auth-tabs {
  display: flex;
  gap: 18px;
  padding: 4px 0;
}
.remember {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.85rem;
  color: var(--muted);
}
.remember input {
  width: auto;
}
.actions {
  display: flex;
  gap: 8px;
}
.actions .primary {
  flex: 1;
}
input {
  padding: 0.6em 0.8em;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--surface-2);
  color: var(--text);
  font-size: 1em;
}
button {
  padding: 0.55em 1em;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
button:hover {
  border-color: #569cd6;
}
.primary {
  background: #0e639c;
  border-color: #0e639c;
}
.primary:hover {
  background: #1177bb;
}
.secondary {
  background: var(--surface-2);
}
.danger {
  background: #3a2525;
  border-color: #654040;
}
.danger:hover {
  border-color: #d16969;
}
.error {
  color: #f48771;
}

.session {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 6px 12px;
  background: var(--surface-3);
  color: var(--text);
  font-size: 13px;
}
.bar-error {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: #f48771;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
.pane {
  min-width: 0;
}
.term-pane {
  flex: 1;
  min-height: 0;
}
.file-pane {
  display: flex;
  flex-direction: column;
  width: 340px;
  border-left: 1px solid var(--line);
}
.sftp-bind {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 38px;
  padding: 6px;
  border-bottom: 1px solid var(--line);
  background: var(--surface);
  font-size: 13px;
}
.sftp-bind span {
  color: var(--muted);
}
.sftp-bind select {
  min-width: 0;
  flex: 1;
  padding: 4px 8px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface-2);
  color: var(--text);
}
.sftp-browser {
  flex: 1;
  min-height: 0;
}
.settings-page {
  max-width: 760px;
  margin: 8vh auto;
  padding: 32px;
}
.settings-section {
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
}
.settings-section + .settings-section {
  margin-top: 14px;
}
.settings-title {
  margin-bottom: 12px;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.segmented {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}
.segmented label {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 36px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-2);
  cursor: pointer;
}
.segmented label.active {
  border-color: #0e639c;
  background: rgba(14, 99, 156, 0.18);
}
.segmented input {
  width: auto;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 54px;
  padding: 10px 0;
}
.setting-row + .setting-row {
  border-top: 1px solid var(--line);
}
.setting-row span {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.setting-row small {
  color: var(--muted);
}
.setting-row input {
  width: auto;
}
.setting-row.disabled {
  opacity: 0.55;
}
@media (max-width: 720px) {
  .machine-page {
    padding: 20px;
  }
  .machine-header,
  .header-actions,
  .actions {
    align-items: stretch;
    flex-direction: column;
  }
  .machine-header .primary,
  .actions .primary {
    flex: none;
  }
  .machine-item {
    grid-template-columns: 1fr;
  }
  .segmented {
    grid-template-columns: 1fr;
  }
}
</style>
