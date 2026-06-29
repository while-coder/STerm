<script setup lang="ts">
// SFTP 面板：标签绑定选择（非跟随模式）+ 文件浏览器。桌面为右侧栏，移动端为全屏视图。
import { useSettings } from "../composables/useSettings";
import { useSessions, type SessionTab } from "../composables/useSessions";
import FileBrowser from "./FileBrowser.vue";

defineProps<{ activeSftpTab: SessionTab }>();

const { settings } = useSettings();
const { tabs, sftpTabId, selectSftpTab, onTabSftpCwdChanged, onTabSftpHomeChanged } = useSessions();
</script>

<template>
  <div class="sftp">
    <div v-if="!settings.sftpFollowSsh" class="bind">
      <span>SFTP</span>
      <select :value="sftpTabId" @change="selectSftpTab(($event.target as HTMLSelectElement).value)">
        <option v-for="tab in tabs" :key="tab.id" :value="tab.id">
          {{ tab.title }} · {{ tab.opts.username }}@{{ tab.opts.host }}
        </option>
      </select>
    </div>
    <div class="browser">
      <FileBrowser
        :key="activeSftpTab.id"
        :id="activeSftpTab.id"
        :session-label="activeSftpTab.title"
        :connected="activeSftpTab.connected"
        :auto-home="settings.sftpAutoHome"
        :follow-path="activeSftpTab.sftpFollowPath"
        :follow-token="activeSftpTab.sftpFollowToken"
        @cwd-changed="onTabSftpCwdChanged(activeSftpTab, $event)"
        @home-changed="onTabSftpHomeChanged(activeSftpTab, $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.sftp {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.bind {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  min-height: 38px;
  padding: var(--sp-2);
  border-bottom: 1px solid var(--line);
  background: var(--surface);
  font-size: var(--fs-sm);
}
.bind span {
  color: var(--muted);
}
.bind select {
  flex: 1;
  min-width: 0;
  min-height: 30px;
  padding: 0 var(--sp-2);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  color: var(--text);
}
.browser {
  flex: 1;
  min-height: 0;
}
</style>
