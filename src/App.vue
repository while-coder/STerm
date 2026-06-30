<script setup lang="ts">
// 应用根：初始化设置（主题监听 + 持久化）并挂载响应式布局骨架。
import { onMounted } from "vue";
import AppLayout from "./components/AppLayout.vue";
import MasterPasswordSetup from "./components/MasterPasswordSetup.vue";
import { useSettings } from "./composables/useSettings";
import { useSecurity } from "./composables/useSecurity";

const { initSettings } = useSettings();
const { initialized, busy, needsSetup, error, initSecurity } = useSecurity();

onMounted(() => {
  initSettings();
  void initSecurity();
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
