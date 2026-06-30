<script setup lang="ts">
// 设置面板：主题、终端外观、SFTP 行为。直接读写 useSettings 单例。
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { appDataDir, join } from "@tauri-apps/api/path";
import { openDir } from "../api";
import { useSettings } from "../composables/useSettings";
import { useSecurity } from "../composables/useSecurity";
import { useGistSync } from "../composables/useGistSync";
import { TERMINAL_FONTS, TERMINAL_SCHEMES } from "../terminalThemes";
import BaseSheet from "./BaseSheet.vue";

const { settings } = useSettings();
const { changeMasterPassword } = useSecurity();
const { syncing, lastError, syncNow, configure, disconnect } = useGistSync();
const patInput = ref("");
const syncMsg = ref("");
const syncHelpOpen = ref(false);

async function openTokenPage() {
  try {
    await openUrl("https://github.com/settings/tokens/new?scopes=gist&description=STerm");
  } catch {
    // 打开浏览器失败时忽略，用户可手动复制下方地址。
  }
}

async function openGistPage() {
  if (!settings.syncGistId) return;
  try {
    await openUrl(`https://gist.github.com/${settings.syncGistId}`);
  } catch {
    // 打开浏览器失败时忽略。
  }
}
const lastSyncText = computed(() =>
  settings.syncLastAt ? new Date(settings.syncLastAt).toLocaleString() : "尚未同步"
);
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

async function onConnectSync() {
  syncMsg.value = "";
  try {
    const login = await configure(patInput.value);
    patInput.value = "";
    syncMsg.value = login ? `已连接 GitHub（${login}）并完成同步` : "已连接并完成同步";
  } catch (e) {
    syncMsg.value = e instanceof Error ? e.message : String(e);
  }
}

async function onSyncNow() {
  syncMsg.value = "";
  try {
    await syncNow();
    syncMsg.value = "同步完成";
  } catch (e) {
    syncMsg.value = e instanceof Error ? e.message : String(e);
  }
}

async function onDisconnectSync() {
  await disconnect();
  syncMsg.value = "已断开同步";
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
      <div class="setting-row">
        <span>
          <strong>配色方案</strong>
          <small>终端前景 / 背景配色。</small>
        </span>
        <select v-model="settings.termColorScheme" class="field">
          <option v-for="s in TERMINAL_SCHEMES" :key="s.key" :value="s.key">{{ s.label }}</option>
        </select>
      </div>
      <div class="setting-row">
        <span>
          <strong>字体</strong>
          <small>等宽字体，需本机已安装。</small>
        </span>
        <select v-model="settings.termFontFamily" class="field">
          <option v-for="f in TERMINAL_FONTS" :key="f.key" :value="f.key">{{ f.label }}</option>
        </select>
      </div>
      <div class="setting-row">
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
      </div>
    </section>

    <section class="section">
      <div class="section-title">安全</div>
      <div class="setting-row security-row">
        <span>
          <strong>主密码</strong>
          <small>修改后会重新加密本地机器列表，并更新系统凭证。</small>
        </span>
        <button class="change-password-btn" type="button" @click="openChangeMasterPassword">
          修改主密码
        </button>
      </div>
      <p v-if="securityMsg" class="setting-msg">{{ securityMsg }}</p>
    </section>

    <section class="section">
      <div class="section-title">云同步</div>
      <template v-if="!settings.syncEnabled">
        <div class="setting-row sync-row">
          <span>
            <strong class="sync-title">
              GitHub Gist 同步
              <button
                class="help-btn"
                type="button"
                title="如何申请 GitHub PAT"
                @click="syncHelpOpen = true"
              >
                ?
              </button>
            </strong>
            <small>
              用带 gist 权限的 Personal Access Token 连接。机器列表以密文上传，主密码不会上云；
              其他设备用同一账号连接时会自动复用同一个 Gist，无需手动填 ID，但需用同一主密码解开。
            </small>
          </span>
        </div>
        <div class="sync-config">
          <input
            v-model="patInput"
            type="password"
            class="field sync-input"
            placeholder="GitHub PAT（需 gist 权限）"
            autocomplete="off"
            @keyup.enter="onConnectSync"
          />
          <button class="change-password-btn" type="button" :disabled="syncing" @click="onConnectSync">
            {{ syncing ? "连接中…" : "连接并同步" }}
          </button>
        </div>
      </template>
      <template v-else>
        <div class="sync-status">
          <div class="sync-status-info">
            <div class="sync-status-line">
              <span class="sync-label">Gist</span>
              <span class="sync-gist-id" :title="settings.syncGistId">
                {{ settings.syncGistId || "（创建中…）" }}
              </span>
            </div>
            <div class="sync-status-line">
              <span class="sync-label">上次同步</span>
              <span>{{ lastSyncText }}</span>
            </div>
          </div>
          <div class="sync-status-actions">
            <button
              class="change-password-btn"
              type="button"
              :disabled="!settings.syncGistId"
              title="在浏览器打开该 Gist（内容为密文）"
              @click="openGistPage"
            >
              在 GitHub 查看
            </button>
            <button class="change-password-btn" type="button" :disabled="syncing" @click="onSyncNow">
              {{ syncing ? "同步中…" : "立即同步" }}
            </button>
            <button class="change-password-btn" type="button" :disabled="syncing" @click="onDisconnectSync">
              断开
            </button>
          </div>
        </div>
      </template>
      <p v-if="syncMsg || lastError" class="setting-msg">{{ syncMsg || lastError }}</p>
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
      <div class="setting-row">
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
      </div>
      <div class="setting-row cache-row">
        <span>
          <strong>查看缓存目录</strong>
          <small>双击查看文件时下载到此目录再用系统程序打开。</small>
        </span>
      </div>
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

    <BaseSheet
      v-if="syncHelpOpen"
      title="如何申请 GitHub PAT"
      subtitle="用于授权 STerm 读写你的 Gist"
      @close="syncHelpOpen = false"
    >
      <div class="help-body">
        <ol class="help-steps">
          <li>登录 GitHub，打开「Settings → Developer settings → Personal access tokens」。</li>
          <li>
            选 <strong>Tokens (classic)</strong> → Generate new token，勾选 <strong>gist</strong> 权限；
            或用 Fine-grained token，在 Account permissions → Gists 设为 <strong>Read and write</strong>。
          </li>
          <li>设置过期时间后生成，<strong>复制 token（只显示一次）</strong>。</li>
          <li>把 token 粘贴到「GitHub PAT」输入框，点「连接并同步」。Gist ID 留空会自动创建。</li>
        </ol>
        <p class="help-hint">下方按钮会打开已预选 gist 权限的令牌创建页：</p>
        <button class="primary help-open-btn" type="button" @click="openTokenPage">
          打开 GitHub 令牌页
        </button>
      </div>
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
.change-password-btn:disabled {
  opacity: 0.7;
  cursor: default;
}
.sync-row {
  min-height: 0;
  padding-bottom: var(--sp-2);
}
.sync-config {
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  padding-bottom: var(--sp-2);
}
.sync-input {
  width: 100%;
  min-height: var(--hit);
}
.sync-status {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
  padding-bottom: var(--sp-2);
}
.sync-status-info {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  font-size: var(--fs-sm);
}
.sync-status-line {
  display: flex;
  align-items: baseline;
  gap: var(--sp-2);
  min-width: 0;
}
.sync-label {
  flex: 0 0 64px;
  color: var(--muted);
}
.sync-gist-id {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--mono, monospace);
  color: var(--text);
}
.sync-status-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--sp-2);
  flex-wrap: wrap;
}
.sync-status-actions button {
  white-space: nowrap;
}
.sync-title {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
}
.help-btn {
  display: inline-grid;
  place-items: center;
  width: 18px;
  height: 18px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: var(--surface-3);
  color: var(--muted);
  font-size: 11px;
  line-height: 1;
  cursor: pointer;
}
.help-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}
.help-body {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.help-steps {
  margin: 0;
  padding-left: 1.2em;
  display: flex;
  flex-direction: column;
  gap: var(--sp-2);
  color: var(--text);
  font-size: var(--fs-sm);
  line-height: 1.6;
}
.help-hint {
  margin: 0;
  color: var(--muted);
  font-size: var(--fs-sm);
}
.help-open-btn {
  min-height: var(--hit);
  padding: 0 var(--sp-4);
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  background: var(--accent);
  color: #fff;
  cursor: pointer;
}
.help-open-btn:hover {
  background: var(--accent-hover);
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
