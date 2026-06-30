<script setup lang="ts">
// 响应式布局骨架：桌面 = 侧栏 + 主区（标签 + 终端 + SFTP）；移动端 = 抽屉 + 全屏视图。
import { computed, ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { SavedConnection } from "../api";
import { useResponsive } from "../composables/useResponsive";
import { useSettings } from "../composables/useSettings";
import { useConnections } from "../composables/useConnections";
import { usePortab } from "../composables/usePortab";
import { useSessions, type SessionTab } from "../composables/useSessions";
import ConnectionSidebar from "./ConnectionSidebar.vue";
import SessionTabs from "./SessionTabs.vue";
import TerminalView from "./TerminalView.vue";
import SftpPanel from "./SftpPanel.vue";
import BaseSheet from "./BaseSheet.vue";
import ConnectionForm from "./ConnectionForm.vue";
import SettingsPanel from "./SettingsPanel.vue";
import PasswordPrompt from "./PasswordPrompt.vue";

const { isMobile } = useResponsive();
const { settings, resolvedTheme } = useSettings();
const { save: saveConn } = useConnections();
const { exportConnections, importConnections } = usePortab();
const {
  tabs,
  activeTabId,
  activeSftpTab,
  openSessionTab,
  activateTab,
  closeTab,
  onTabConnected,
  onTabCwd,
} = useSessions();

// —— 浮层与导航状态 ——
const formOpen = ref(false);
const editing = ref<SavedConnection | null>(null);
const formError = ref("");
const settingsOpen = ref(false);
const drawerOpen = ref(false);
const pickerOpen = ref(false);
const mobileView = ref<"term" | "sftp">("term");

// —— 导入 / 导出 ——
const passwordPrompt = ref<{ open: boolean; mode: "export" | "import"; path: string }>({
  open: false,
  mode: "export",
  path: "",
});
const portabBusy = ref(false);
const portabMsg = ref("");

// 导出：先选保存路径，再弹口令对话框。
async function onExport() {
  drawerOpen.value = false;
  const path = await save({
    title: "导出机器列表",
    defaultPath: "sterm-connections.json",
    filters: [{ name: "STerm 导出文件", extensions: ["json"] }],
  });
  if (typeof path !== "string") return;
  passwordPrompt.value = { open: true, mode: "export", path };
}

// 导入：先选文件，再弹口令对话框。
async function onImport() {
  drawerOpen.value = false;
  const path = await open({
    multiple: false,
    title: "导入机器列表",
    filters: [{ name: "STerm 导出文件", extensions: ["json"] }],
  });
  if (typeof path !== "string") return;
  passwordPrompt.value = { open: true, mode: "import", path };
}

// 口令确认：执行实际导入 / 导出。
async function onPortabConfirm(password: string) {
  const { mode, path } = passwordPrompt.value;
  passwordPrompt.value.open = false;
  portabBusy.value = true;
  portabMsg.value = "";
  try {
    const res =
      mode === "export"
        ? await exportConnections(password, path)
        : await importConnections(password, path);
    const action = mode === "export" ? "导出" : "导入";
    let msg = `已${action} ${res.count} 条连接`;
    if (res.warnings.length) msg += `（${res.warnings.length} 项告警：${res.warnings.join("；")}）`;
    portabMsg.value = msg;
  } catch (e) {
    portabMsg.value = `操作失败：${e instanceof Error ? e.message : String(e)}`;
  } finally {
    portabBusy.value = false;
  }
}

const showSftp = computed(() => settings.showSftp && !!activeSftpTab.value);
const showSidebar = computed(
  () => !isMobile.value && !settings.sidebarCollapsed && tabs.value.length > 0
);

// —— 连接 ——
function doConnect(conn: SavedConnection) {
  openSessionTab(conn);
  formOpen.value = false;
  drawerOpen.value = false;
  pickerOpen.value = false;
  mobileView.value = "term";
}

function handleConnect(conn: SavedConnection) {
  // 密码认证但未保存密码 → 打开表单补填。
  if (conn.auth === "password" && !conn.password) {
    openEdit(conn);
    formError.value = "请输入密码后连接";
    return;
  }
  doConnect(conn);
}

function openNew() {
  editing.value = null;
  formError.value = "";
  formOpen.value = true;
  drawerOpen.value = false;
  pickerOpen.value = false;
}

function openEdit(conn: SavedConnection) {
  editing.value = conn;
  formError.value = "";
  formOpen.value = true;
  drawerOpen.value = false;
  pickerOpen.value = false;
}

function onFormSave(conn: SavedConnection) {
  saveConn(conn);
  formOpen.value = false;
}

function onFormConnect(conn: SavedConnection) {
  saveConn(conn);
  doConnect(conn);
}

// 连接失败：未建立的会话回到表单补填，并关闭该标签。
function onTabError(tab: SessionTab, msg: string) {
  tab.error = msg;
  if (!tab.connected) {
    const conn: SavedConnection = {
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
    };
    closeTab(tab.id);
    editing.value = conn;
    formError.value = msg;
    formOpen.value = true;
  }
}

// —— 侧栏宽度拖拽（桌面）——
function startResizeSidebar(e: PointerEvent) {
  const startX = e.clientX;
  const startW = settings.sidebarWidth;
  const move = (ev: PointerEvent) => {
    const w = startW + (ev.clientX - startX);
    settings.sidebarWidth = Math.min(480, Math.max(180, w));
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

// —— SFTP 面板宽度拖拽（桌面）——
function startResize(e: PointerEvent) {
  const startX = e.clientX;
  const startW = settings.sftpPanelWidth;
  const move = (ev: PointerEvent) => {
    const w = startW - (ev.clientX - startX);
    settings.sftpPanelWidth = Math.min(640, Math.max(220, w));
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}
</script>

<template>
  <div
    class="layout"
    :class="{ mobile: isMobile, 'has-sidebar': showSidebar }"
    :style="showSidebar ? { gridTemplateColumns: `${settings.sidebarWidth}px 6px 1fr` } : undefined"
  >
    <!-- 侧栏：桌面常驻（可折叠，分割线可拖拽调宽）；移动端为抽屉。 -->
    <div v-if="showSidebar" class="sidebar-col">
      <ConnectionSidebar
        @connect="handleConnect"
        @edit="openEdit"
        @new="openNew"
        @settings="settingsOpen = true"
        @export="onExport"
        @import="onImport"
      />
    </div>
    <div
      v-if="showSidebar"
      class="sidebar-resizer"
      title="拖拽调整侧栏宽度"
      @pointerdown.prevent="startResizeSidebar"
    ></div>

    <!-- 移动端抽屉 -->
    <div v-if="isMobile && drawerOpen" class="drawer-backdrop" @click.self="drawerOpen = false">
      <div class="drawer">
        <ConnectionSidebar
          mobile
          @connect="handleConnect"
          @edit="openEdit"
          @new="openNew"
          @settings="settingsOpen = true"
          @export="onExport"
          @import="onImport"
          @close="drawerOpen = false"
        />
      </div>
    </div>

    <!-- 主区 -->
    <div class="main">
      <!-- 无会话：主区直接呈现完整机器列表（启动即列表页）。 -->
      <template v-if="!tabs.length">
        <ConnectionSidebar
          home
          class="home-list"
          @connect="handleConnect"
          @edit="openEdit"
          @new="openNew"
          @settings="settingsOpen = true"
          @export="onExport"
          @import="onImport"
        />
      </template>

      <template v-else>
        <SessionTabs
          :tabs="tabs"
          :active-tab-id="activeTabId"
          :mobile="isMobile"
          @activate="activateTab"
          @close="closeTab"
          @list="pickerOpen = true"
          @menu="isMobile ? (drawerOpen = true) : (settings.sidebarCollapsed = !settings.sidebarCollapsed)"
        />

        <!-- 移动端：终端 / 文件 切换条 -->
        <div v-if="isMobile && showSftp" class="mobile-switch">
          <button :class="{ active: mobileView === 'term' }" type="button" @click="mobileView = 'term'">终端</button>
          <button :class="{ active: mobileView === 'sftp' }" type="button" @click="mobileView = 'sftp'">文件</button>
        </div>

        <div class="body">
          <div
            v-show="!isMobile || !showSftp || mobileView === 'term'"
            class="term-col"
          >
            <section
              v-for="tab in tabs"
              v-show="tab.id === activeTabId"
              :key="tab.id"
              class="term-host"
            >
              <TerminalView
                :opts="tab.opts"
                :active="tab.id === activeTabId"
                :theme="resolvedTheme"
                @connected="onTabConnected(tab)"
                @error="onTabError(tab, $event)"
                @closed="closeTab(tab.id)"
                @cwd="onTabCwd(tab, $event)"
              />
            </section>
          </div>

          <template v-if="showSftp && activeSftpTab">
            <div
              v-if="!isMobile"
              class="resizer"
              title="拖拽调整宽度"
              @pointerdown.prevent="startResize"
            ></div>
            <div
              v-show="!isMobile || mobileView === 'sftp'"
              class="sftp-col"
              :style="!isMobile ? { width: settings.sftpPanelWidth + 'px' } : undefined"
            >
              <SftpPanel :active-sftp-tab="activeSftpTab" />
            </div>
          </template>
        </div>
      </template>
    </div>

    <!-- 连接表单 -->
    <BaseSheet
      v-if="formOpen"
      :title="editing && editing.id ? '编辑连接' : '新建连接'"
      subtitle="填写机器信息后可直接连接，或保存到列表"
      @close="formOpen = false"
    >
      <ConnectionForm
        :connection="editing"
        :error="formError"
        @save="onFormSave"
        @connect="onFormConnect"
        @cancel="formOpen = false"
      />
    </BaseSheet>

    <!-- 机器列表选择器（标签栏 ＋ 打开） -->
    <BaseSheet
      v-if="pickerOpen"
      title="机器列表"
      subtitle="选择一台机器连接，或新建连接"
      @close="pickerOpen = false"
    >
      <ConnectionSidebar
        class="picker-list"
        home
        hide-header
        @connect="handleConnect"
        @edit="openEdit"
        @new="openNew"
      />
    </BaseSheet>

    <!-- 设置 -->
    <BaseSheet v-if="settingsOpen" title="设置" subtitle="调整界面和 SFTP 行为" @close="settingsOpen = false">
      <SettingsPanel />
    </BaseSheet>

    <!-- 导入 / 导出口令 -->
    <PasswordPrompt
      v-if="passwordPrompt.open"
      :mode="passwordPrompt.mode"
      @confirm="onPortabConfirm"
      @cancel="passwordPrompt.open = false"
    />

    <!-- 导入 / 导出结果提示 -->
    <div v-if="portabBusy || portabMsg" class="portab-toast" @click="portabMsg = ''">
      {{ portabBusy ? "处理中…" : portabMsg }}
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 1fr;
  height: 100vh;
  min-height: 0;
  padding-left: var(--safe-left);
  padding-right: var(--safe-right);
}
.layout.has-sidebar {
  grid-template-columns: var(--sidebar-w) 1fr;
}
.sidebar-col {
  min-width: 0;
  min-height: 0;
  padding-top: var(--safe-top);
}
.main {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}
.layout:not(.mobile) .main {
  padding-top: var(--safe-top);
}
.body {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.term-col {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
}
.term-host {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.resizer {
  width: 6px;
  flex: 0 0 auto;
  cursor: col-resize;
  background: var(--line);
}
.resizer:hover {
  background: var(--accent);
}
.sidebar-resizer {
  min-height: 0;
  cursor: col-resize;
  background: var(--line);
}
.sidebar-resizer:hover {
  background: var(--accent);
}
.sftp-col {
  flex: 0 0 auto;
  min-height: 0;
  border-left: 1px solid var(--line);
}
.layout.mobile .sftp-col {
  flex: 1;
  width: 100% !important;
  border-left: 0;
}
.layout.mobile .term-col {
  width: 100%;
}

/* 移动端抽屉 */
.drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 40;
  background: var(--overlay);
}
.drawer {
  width: min(82vw, 320px);
  height: 100%;
  padding-top: var(--safe-top);
  padding-bottom: var(--safe-bottom);
  background: var(--surface);
  box-shadow: var(--shadow);
}
.home-list {
  height: 100%;
}
/* 选择器内的列表：撑开内容由 sheet 滚动，去掉首项多余留白。 */
.picker-list {
  height: auto;
  margin: calc(var(--sp-3) * -1) 0;
}

/* 导入 / 导出结果提示 */
.portab-toast {
  position: fixed;
  left: 50%;
  bottom: calc(var(--safe-bottom) + var(--sp-5));
  transform: translateX(-50%);
  z-index: 60;
  max-width: min(90vw, 560px);
  padding: var(--sp-3) var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-3);
  color: var(--text);
  font-size: var(--fs-sm);
  box-shadow: var(--shadow);
  cursor: pointer;
}

/* 移动端终端 / 文件切换 */
.mobile-switch {
  display: flex;
  gap: var(--sp-2);
  padding: var(--sp-2);
  border-bottom: 1px solid var(--line);
  background: var(--surface-2);
}
.mobile-switch button {
  flex: 1;
  min-height: 36px;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-3);
  color: var(--muted);
  cursor: pointer;
}
.mobile-switch button.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--text);
}

</style>
