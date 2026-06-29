<script setup lang="ts">
// 会话标签条：桌面横向滚动标签；移动端紧凑、可横滑。
import type { SessionTab } from "../composables/useSessions";

defineProps<{
  tabs: SessionTab[];
  activeTabId: string;
  mobile?: boolean;
}>();
const emit = defineEmits<{
  activate: [id: string];
  close: [id: string];
  list: [];
  menu: [];
}>();
</script>

<template>
  <div class="tabbar">
    <button class="lead" type="button" title="连接列表" @click="emit('menu')">☰</button>
    <div class="strip">
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="tab"
        :class="{ active: tab.id === activeTabId }"
        :title="`${tab.title} · ${tab.opts.username}@${tab.opts.host}:${tab.opts.port}`"
        @click="emit('activate', tab.id)"
      >
        <span class="dot" :class="{ ok: tab.connected, err: tab.error }"></span>
        <span class="title">{{ tab.title }}</span>
        <button class="close" type="button" title="关闭" @click.stop="emit('close', tab.id)">×</button>
      </div>
      <button class="add" type="button" title="机器列表" @click="emit('list')">＋</button>
    </div>
  </div>
</template>

<style scoped>
.tabbar {
  display: flex;
  align-items: stretch;
  height: var(--topbar-h);
  background: var(--surface-2);
  border-bottom: 1px solid var(--line);
}
.lead {
  flex: 0 0 auto;
  width: var(--topbar-h);
  border: 0;
  border-right: 1px solid var(--line);
  background: transparent;
  color: var(--muted);
  font-size: 18px;
  cursor: pointer;
}
.add {
  flex: 0 0 auto;
  width: var(--topbar-h);
  border: 0;
  background: transparent;
  color: var(--muted);
  font-size: 18px;
  cursor: pointer;
}
.lead:hover,
.add:hover {
  background: var(--surface-3);
  color: var(--text);
}
.strip {
  display: flex;
  align-items: stretch;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: thin;
}
.tab {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  min-width: 130px;
  max-width: 220px;
  padding: 0 var(--sp-2) 0 var(--sp-3);
  border-right: 1px solid var(--line);
  color: var(--muted);
  background: var(--surface-2);
  cursor: pointer;
}
.tab:hover {
  background: var(--surface-3);
}
.tab.active {
  background: var(--bg);
  color: var(--text);
}
.dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: #777;
}
.dot.ok {
  background: var(--ok);
}
.dot.err {
  background: var(--warn);
}
.title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fs-sm);
  font-weight: 600;
}
.close {
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 15px;
  cursor: pointer;
}
.close:hover {
  background: var(--surface-2);
  color: var(--text);
}
</style>
