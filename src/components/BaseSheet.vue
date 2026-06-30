<script setup lang="ts">
// 通用浮层：桌面=居中卡片 + 背景遮罩；移动端=全屏 sheet。点击遮罩或 Esc 关闭。
import { onBeforeUnmount, onMounted } from "vue";
import { useResponsive } from "../composables/useResponsive";

const props = defineProps<{ title?: string; subtitle?: string }>();
const emit = defineEmits<{ close: [] }>();
const { isMobile } = useResponsive();

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="sheet-backdrop" :class="{ mobile: isMobile }" @click.self="emit('close')">
    <div class="sheet" :class="{ mobile: isMobile }" role="dialog" aria-modal="true">
      <header v-if="props.title" class="sheet-head">
        <div class="sheet-titles">
          <h2>{{ props.title }}</h2>
          <p v-if="props.subtitle" class="sheet-sub">{{ props.subtitle }}</p>
        </div>
        <button class="sheet-close" type="button" title="关闭" @click="emit('close')">×</button>
      </header>
      <div class="sheet-body">
        <slot />
      </div>
    </div>
  </div>
</template>

<style scoped>
.sheet-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: calc(var(--safe-top) + var(--sp-4)) var(--sp-4) calc(var(--safe-bottom) + var(--sp-4));
  background: var(--overlay);
}
.sheet-backdrop.mobile {
  padding: 0;
  align-items: stretch;
}
.sheet {
  width: min(520px, 100%);
  max-height: calc(100vh - var(--sp-6) * 2);
  min-width: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--line);
  border-radius: var(--radius-lg);
  background: var(--bg);
  box-shadow: var(--shadow);
  overflow: hidden;
}
.sheet.mobile {
  width: 100%;
  max-height: none;
  height: 100%;
  border: 0;
  border-radius: 0;
  padding-top: var(--safe-top);
  padding-bottom: var(--safe-bottom);
}
.sheet-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--sp-3);
  padding: var(--sp-4) var(--sp-5);
  border-bottom: 1px solid var(--line);
}
.sheet-titles h2 {
  margin: 0;
  font-size: var(--fs-lg);
}
.sheet-sub {
  margin: var(--sp-1) 0 0;
  color: var(--muted);
  font-size: var(--fs-sm);
}
.sheet-close {
  width: 32px;
  height: 32px;
  flex: 0 0 auto;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 22px;
  line-height: 1;
  cursor: pointer;
}
.sheet-close:hover {
  background: var(--surface-3);
  color: var(--text);
}
.sheet-body {
  padding: var(--sp-5);
  min-width: 0;
  overflow-y: auto;
  overflow-x: hidden;
}
</style>
