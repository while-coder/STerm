<script setup lang="ts">
// 应用根：初始化设置（主题监听 + 持久化）并挂载响应式布局骨架。
import { onMounted } from "vue";
import AppLayout from "./components/AppLayout.vue";
import MasterPasswordSetup from "./components/MasterPasswordSetup.vue";
import { useSettings } from "./composables/useSettings";
import { useSecurity } from "./composables/useSecurity";
import { useGistSync } from "./composables/useGistSync";

const { initSettings, settings } = useSettings();
const { initialized, busy, needsSetup, error, initSecurity } = useSecurity();
const { syncNow } = useGistSync();

onMounted(async () => {
  initSettings();
  await initSecurity();
  // 已解锁且开启同步时，启动后拉取一次（失败只记录，不阻断使用）。
  if (!needsSetup.value && settings.syncEnabled && settings.syncGistId) {
    void syncNow().catch(() => undefined);
  }
});
</script>

<template>
  <div v-if="!initialized || busy" class="boot">正在加载 STerm…</div>
  <MasterPasswordSetup v-else-if="needsSetup" :initial-error="error" />
  <AppLayout v-else />
</template>

<style scoped>
.boot {
  display: grid;
  place-items: center;
  height: 100vh;
  color: var(--muted);
  background: var(--bg);
}
</style>
