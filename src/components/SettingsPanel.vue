<script setup lang="ts">
// 设置面板：主题、终端外观、SFTP 行为。直接读写 useSettings 单例。
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { appDataDir, join } from "@tauri-apps/api/path";
import { openDir } from "../api";
import { useSettings } from "../composables/useSettings";
import { useSecurity } from "../composables/useSecurity";
import { TERMINAL_FONTS, TERMINAL_SCHEMES } from "../terminalThemes";
import BaseSheet from "./BaseSheet.vue";

const { settings } = useSettings();
const { changeMasterPassword } = useSecurity();
const changePasswordOpen = ref(false);
const newMasterPassword = ref("");
const confirmMasterPassword = ref("");
const securityBusy = ref(false);
const securityMsg = ref("");
const passwordDialogError = ref("");
const defaultCacheDir = ref("");
const cacheDirMsg = ref("");
const effectiveCacheDir = computed(() => settings.sftpCacheDir.trim() || defaultCacheDir.value);

function clampFontSize() {
  settings.termFontSize = Math.min(28, Math.max(8, Math.round(settings.termFontSize) || 14));
}
function clampParallel() {
  settings.maxParallelTransfers = Math.min(8, Math.max(1, Math.round(settings.maxParallelTransfers) || 1));
}
// 选择双击查看时的本地缓存目录。
async function pickCacheDir() {
  const dir = await open({ directory: true });
  if (dir && typeof dir === "string") {
    settings.sftpCacheDir = dir;
    cacheDirMsg.value = "";
  }
}

async function openCacheDir() {
  const dir = effectiveCacheDir.value;
  if (!dir) return;
  cacheDirMsg.value = "";
  try {
    await openDir(dir);
  } catch (e) {
    cacheDirMsg.value = e instanceof Error ? e.message : String(e);
  }
}

function resetCacheDir() {
  settings.sftpCacheDir = "";
  cacheDirMsg.value = "已重置为应用缓存目录";
}

function openChangeMasterPassword() {
  newMasterPassword.value = "";
  confirmMasterPassword.value = "";
  passwordDialogError.value = "";
  securityMsg.value = "";
  changePasswordOpen.value = true;
}

async function onChangeMasterPassword() {
  if (!newMasterPassword.value) {
    passwordDialogError.value = "请输入新的主密码";
    return;
  }
  if (newMasterPassword.value !== confirmMasterPassword.value) {
    passwordDialogError.value = "两次输入的主密码不一致";
    return;
  }
  securityBusy.value = true;
  passwordDialogError.value = "";
  try {
    await changeMasterPassword(newMasterPassword.value);
    newMasterPassword.value = "";
    confirmMasterPassword.value = "";
    changePasswordOpen.value = false;
    securityMsg.value = "主密码已更新";
  } catch (e) {
    passwordDialogError.value = e instanceof Error ? e.message : String(e);
  } finally {
    securityBusy.value = false;
  }
}

onMounted(async () => {
  try {
    defaultCacheDir.value = await join(await appDataDir(), "viewer-cache");
  } catch (e) {
    cacheDirMsg.value = e instanceof Error ? e.message : String(e);
  }
});
</script>

<template>
  <div class="settings">
    <section class="section">
      <div class="section-title">主题</div>
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

    <section class="section">
      <div class="section-title">终端</div>
      <label class="setting-row">
        <span>
          <strong>配色方案</strong>
          <small>终端前景 / 背景配色。</small>
        </span>
        <select v-model="settings.termColorScheme" class="field">
          <option v-for="s in TERMINAL_SCHEMES" :key="s.key" :value="s.key">{{ s.label }}</option>
        </select>
      </label>
      <label class="setting-row">
        <span>
          <strong>字体</strong>
          <small>等宽字体，需本机已安装。</small>
        </span>
        <select v-model="settings.termFontFamily" class="field">
          <option v-for="f in TERMINAL_FONTS" :key="f.key" :value="f.key">{{ f.label }}</option>
        </select>
      </label>
      <label class="setting-row">
        <span>
          <strong>字体大小</strong>
          <small>8 – 28 px。</small>
        </span>
        <span class="stepper">
          <button type="button" @click="settings.termFontSize--; clampFontSize()">−</button>
          <input
            v-model.number="settings.termFontSize"
            type="number"
            min="8"
            max="28"
            @change="clampFontSize"
          />
          <button type="button" @click="settings.termFontSize++; clampFontSize()">＋</button>
        </span>
      </label>
    </section>

    <section class="section">
      <div class="section-title">安全</div>
      <label class="setting-row security-row">
        <span>
          <strong>主密码</strong>
          <small>修改后会重新加密本地机器列表，并更新系统凭证。</small>
        </span>
        <button class="change-password-btn" type="button" @click="openChangeMasterPassword">
          修改主密码
        </button>
      </label>
      <p v-if="securityMsg" class="setting-msg">{{ securityMsg }}</p>
    </section>

    <section class="section">
      <div class="section-title">SFTP</div>
      <label class="setting-row">
        <span>
          <strong>显示 SFTP 文件面板</strong>
          <small>关闭后只显示 SSH 终端。</small>
        </span>
        <input v-model="settings.showSftp" type="checkbox" />
      </label>
      <label class="setting-row" :class="{ disabled: !settings.showSftp }">
        <span>
          <strong>SFTP 跟随终端目录</strong>
          <small>终端里 cd 切换目录时，文件面板自动定位到当前工作目录。</small>
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
      <label class="setting-row">
        <span>
          <strong>并行传输上限</strong>
          <small>同时进行的上传 / 下载数，其余排队。</small>
        </span>
        <span class="stepper">
          <button type="button" @click="settings.maxParallelTransfers--; clampParallel()">−</button>
          <input
            v-model.number="settings.maxParallelTransfers"
            type="number"
            min="1"
            max="8"
            @change="clampParallel"
          />
          <button type="button" @click="settings.maxParallelTransfers++; clampParallel()">＋</button>
        </span>
      </label>
      <label class="setting-row cache-row">
        <span>
          <strong>查看缓存目录</strong>
          <small>双击查看文件时下载到此目录再用系统程序打开。</small>
        </span>
      </label>
      <div class="cache-dir-control">
        <div class="cache-dir-path" :title="effectiveCacheDir">
          {{ effectiveCacheDir || "正在获取应用数据目录…" }}
        </div>
        <div class="cache-dir-actions">
          <button type="button" title="打开当前缓存目录" @click="openCacheDir">打开目录</button>
          <button type="button" title="选择缓存目录" @click="pickCacheDir">选择目录</button>
          <button type="button" title="重置为应用缓存目录" @click="resetCacheDir">重置</button>
        </div>
      </div>
      <p v-if="cacheDirMsg" class="setting-msg">{{ cacheDirMsg }}</p>
    </section>

    <BaseSheet
      v-if="changePasswordOpen"
      title="修改主密码"
      subtitle="保存后会重新加密本地机器列表，并更新系统凭证"
      @close="changePasswordOpen = false"
    >
      <form class="password-form" @submit.prevent="onChangeMasterPassword">
        <input
          v-model="newMasterPassword"
          type="password"
          placeholder="新主密码"
          autocomplete="new-password"
        />
        <input
          v-model="confirmMasterPassword"
          type="password"
          placeholder="再次输入新主密码"
          autocomplete="new-password"
        />
        <p v-if="passwordDialogError" class="error">{{ passwordDialogError }}</p>
        <div class="dialog-actions">
          <button type="button" :disabled="securityBusy" @click="changePasswordOpen = false">
            取消
          </button>
          <button type="submit" class="primary" :disabled="securityBusy">
            {{ securityBusy ? "保存中…" : "保存" }}
          </button>
        </div>
      </form>
    </BaseSheet>
  </div>
</template>

<style scoped>
.settings {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.section {
  padding: var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface);
}
.section-title {
  margin-bottom: var(--sp-3);
  font-size: var(--fs-xs);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.segmented {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--sp-2);
}
.segmented label {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-2);
  min-height: var(--hit);
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  cursor: pointer;
}
.segmented label.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.segmented input {
  display: none;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  min-width: 0;
  min-height: 54px;
  padding: var(--sp-3) 0;
}
.setting-row + .setting-row {
  border-top: 1px solid var(--line);
}
.setting-row span {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  min-width: 0;
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
.field {
  flex: 0 0 auto;
  min-width: 160px;
  min-height: 34px;
  padding: 0 var(--sp-2);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  color: var(--text);
}
.stepper {
  flex-direction: row !important;
  align-items: center;
  gap: var(--sp-1);
}
.stepper button {
  width: 32px;
  height: 32px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.stepper button:hover {
  border-color: var(--accent);
}
.stepper input {
  width: 52px;
  min-height: 32px;
  text-align: center;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  color: var(--text);
}
.cache-row {
  min-height: 0;
  padding-bottom: var(--sp-2);
}
.cache-dir-control {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  padding-bottom: var(--sp-3);
}
.cache-dir-path {
  min-width: 0;
  min-height: 32px;
  padding: 6px var(--sp-2);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-sm);
  line-height: 1.45;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.cache-dir-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--sp-1);
  white-space: nowrap;
}
.cache-dir-actions button {
  min-height: 32px;
  padding: 0 var(--sp-2);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
  white-space: nowrap;
}
.cache-dir-actions button:hover {
  border-color: var(--accent);
}
.change-password-btn {
  min-height: 32px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.change-password-btn:hover {
  border-color: var(--accent);
}
.setting-msg {
  margin: 0;
  color: var(--muted);
  font-size: var(--fs-sm);
}
.password-form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.password-form input {
  min-height: var(--hit);
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-md);
}
.dialog-actions {
  display: flex;
  gap: var(--sp-2);
  margin-top: var(--sp-1);
}
.dialog-actions button {
  min-height: var(--hit);
  padding: 0 var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.dialog-actions button:hover {
  border-color: var(--accent);
}
.dialog-actions button:disabled {
  opacity: 0.7;
  cursor: default;
}
.dialog-actions .primary {
  flex: 1;
  border-color: var(--accent);
  background: var(--accent);
  color: #fff;
}
.dialog-actions .primary:not(:disabled):hover {
  background: var(--accent-hover);
}
.error {
  margin: 0;
  color: var(--warn);
  font-size: var(--fs-sm);
}
</style>
