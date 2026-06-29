<script setup lang="ts">
// 通用上下文菜单：teleport 到 body，按光标定位并做视口边缘翻转，点击外部 / Esc 关闭。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

export interface MenuItem {
  key: string;
  label: string;
  danger?: boolean;
}

const props = defineProps<{
  open: boolean;
  x: number;
  y: number;
  items: MenuItem[];
}>();
const emit = defineEmits<{ select: [key: string]; close: [] }>();

const el = ref<HTMLElement | null>(null);
const pos = ref({ x: 0, y: 0 });

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    await nextTick();
    const w = el.value?.offsetWidth ?? 160;
    const h = el.value?.offsetHeight ?? 0;
    pos.value = {
      x: Math.max(8, Math.min(props.x, window.innerWidth - w - 8)),
      y: Math.max(8, Math.min(props.y, window.innerHeight - h - 8)),
    };
  }
);

function onDocPointer(e: PointerEvent) {
  if (!props.open) return;
  if (el.value && !el.value.contains(e.target as Node)) emit("close");
}
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

onMounted(() => {
  document.addEventListener("pointerdown", onDocPointer, true);
  window.addEventListener("keydown", onKey);
});
onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onDocPointer, true);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="el"
      class="ctx"
      :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
    >
      <button
        v-for="it in items"
        :key="it.key"
        class="ci"
        :class="{ danger: it.danger }"
        type="button"
        @click="emit('select', it.key); emit('close')"
      >
        {{ it.label }}
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.ctx {
  position: fixed;
  z-index: 200;
  display: flex;
  flex-direction: column;
  min-width: 140px;
  padding: var(--sp-1);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--bg);
  box-shadow: var(--shadow);
}
.ci {
  padding: var(--sp-2) var(--sp-3);
  min-height: 36px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  text-align: left;
  font-size: var(--fs-sm);
  cursor: pointer;
}
.ci:hover {
  background: var(--surface-2);
}
.ci.danger {
  color: var(--danger);
}
</style>
