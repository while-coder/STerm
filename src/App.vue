<script setup lang="ts">
// 应用根：初始化设置（主题监听 + 持久化）并挂载响应式布局骨架。
import { onMounted } from "vue";
import AppLayout from "./components/AppLayout.vue";
import { useSettings } from "./composables/useSettings";

const { initSettings } = useSettings();
onMounted(initSettings);
</script>

<template>
  <AppLayout />
</template>
