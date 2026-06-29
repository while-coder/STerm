<script setup lang="ts">
// 设置面板：主题 + SFTP 行为。直接读写 useSettings 单例。
import { useSettings } from "../composables/useSettings";

const { settings } = useSettings();
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
    </section>
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
</style>
