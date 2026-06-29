<script setup lang="ts">
// 侧栏单条连接：主体点击连接，右侧常显图标操作；右键 / 长按弹出上下文菜单。
import type { SavedConnection } from "../api";
import { useLongPress } from "../composables/useLongPress";

const props = defineProps<{ conn: SavedConnection }>();
const emit = defineEmits<{
  connect: [conn: SavedConnection];
  edit: [conn: SavedConnection];
  remove: [conn: SavedConnection];
  favorite: [conn: SavedConnection];
  context: [payload: { conn: SavedConnection; x: number; y: number }];
}>();

const lp = useLongPress();
const needsPassword = () => props.conn.auth === "password" && !props.conn.password;

function onMain() {
  if (lp.suppressed.value) return;
  emit("connect", props.conn);
}
function openCtx(e: MouseEvent | PointerEvent) {
  emit("context", { conn: props.conn, x: e.clientX, y: e.clientY });
}
</script>

<template>
  <div class="item" @contextmenu.prevent="openCtx">
    <button
      class="main"
      type="button"
      :title="`${conn.label} (${conn.host}:${conn.port})`"
      @click="onMain"
      @pointerdown="lp.start($event, openCtx)"
      @pointermove="lp.move"
      @pointerup="lp.end"
      @pointercancel="lp.end"
    >
      <span class="name">{{ conn.label }}</span>
      <span class="meta">
        {{ conn.host }}:{{ conn.port }} · {{ conn.auth === "key" ? "私钥" : "密码" }}
        <em v-if="needsPassword()" class="hint">需密码</em>
      </span>
    </button>
    <div class="actions">
      <button
        class="icon"
        :class="{ on: conn.favorite }"
        type="button"
        title="收藏"
        @click.stop="emit('favorite', conn)"
      >
        ★
      </button>
      <button class="icon" type="button" title="编辑" @click.stop="emit('edit', conn)">✎</button>
      <button class="icon danger" type="button" title="删除" @click.stop="emit('remove', conn)">🗑</button>
    </div>
  </div>
</template>

<style scoped>
.item {
  display: flex;
  align-items: stretch;
  gap: var(--sp-1);
  border-radius: var(--radius);
  background: transparent;
}
.item:hover {
  background: var(--surface-2);
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 2px;
  min-height: var(--hit);
  padding: var(--sp-2) var(--sp-3);
  border: 0;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text);
  text-align: left;
  cursor: pointer;
}
.name {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fs-md);
  font-weight: 600;
}
.meta {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fs-xs);
  color: var(--muted);
}
.hint {
  font-style: normal;
  color: var(--warn);
}
.actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-right: var(--sp-1);
  opacity: 0;
  transition: opacity 0.12s;
}
.item:hover .actions,
.actions:focus-within {
  opacity: 1;
}
.icon {
  width: 30px;
  height: 30px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 14px;
  cursor: pointer;
}
.icon:hover {
  background: var(--surface-3);
  color: var(--text);
}
.icon.on {
  color: #f5c451;
  opacity: 1;
}
.icon.danger:hover {
  color: var(--danger);
}
/* 触控设备：操作常显，无 hover。 */
@media (hover: none) {
  .actions {
    opacity: 1;
  }
}
</style>
