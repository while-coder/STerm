<script setup lang="ts">
// 连接侧栏：搜索、收藏置顶、按分组聚合的连接列表、新建入口；右键 / 长按弹出操作菜单。
import { computed, ref } from "vue";
import type { SavedConnection } from "../api";
import { useConnections } from "../composables/useConnections";
import ConnectionItem from "./ConnectionItem.vue";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";

defineProps<{ mobile?: boolean; home?: boolean; hideHeader?: boolean }>();
const emit = defineEmits<{
  connect: [conn: SavedConnection];
  edit: [conn: SavedConnection];
  new: [];
  close: [];
  settings: [];
  export: [];
  import: [];
}>();

const { connections, query, grouped, remove, toggleFavorite } = useConnections();

// 上下文菜单状态。
const menu = ref<{ open: boolean; x: number; y: number; conn: SavedConnection | null }>({
  open: false,
  x: 0,
  y: 0,
  conn: null,
});
const menuItems = computed<MenuItem[]>(() => {
  const c = menu.value.conn;
  if (!c) return [];
  return [
    { key: "connect", label: "连接" },
    { key: "edit", label: "编辑" },
    { key: "favorite", label: c.favorite ? "取消收藏" : "收藏" },
    { key: "remove", label: "删除", danger: true },
  ];
});
function openMenu(payload: { conn: SavedConnection; x: number; y: number }) {
  menu.value = { open: true, x: payload.x, y: payload.y, conn: payload.conn };
}
function onMenuSelect(key: string) {
  const c = menu.value.conn;
  if (!c) return;
  if (key === "connect") emit("connect", c);
  else if (key === "edit") emit("edit", c);
  else if (key === "favorite") toggleFavorite(c);
  else if (key === "remove") remove(c.id);
}
</script>

<template>
  <aside class="sidebar" :class="{ home }">
    <header v-if="!hideHeader" class="head">
      <div class="brand">STerm</div>
      <div class="head-actions">
        <button class="icon" type="button" title="设置" @click="emit('settings')">⚙</button>
        <button
          v-if="mobile"
          class="icon"
          type="button"
          title="收起"
          @click="emit('close')"
        >
          ×
        </button>
      </div>
    </header>

    <div v-if="!hideHeader" class="quick-actions" aria-label="机器列表导入导出">
      <button class="tool-btn" type="button" title="导入机器列表" @click="emit('import')">
        <span class="tool-icon">↓</span>
        <span>导入</span>
      </button>
      <button class="tool-btn" type="button" title="导出机器列表" @click="emit('export')">
        <span class="tool-icon">↑</span>
        <span>导出</span>
      </button>
    </div>

    <div class="search">
      <input v-model="query" type="search" placeholder="搜索机器…" />
    </div>

    <button class="new" type="button" @click="emit('new')">＋ 新建连接</button>

    <div v-if="connections.length" class="list">
      <template v-if="grouped.favorites.length">
        <div class="group-title">★ 收藏</div>
        <ConnectionItem
          v-for="c in grouped.favorites"
          :key="`fav-${c.id}`"
          :conn="c"
          @connect="emit('connect', $event)"
          @edit="emit('edit', $event)"
          @remove="remove($event.id)"
          @favorite="toggleFavorite($event)"
          @context="openMenu"
        />
      </template>

      <template v-for="g in grouped.groups" :key="g.name">
        <div class="group-title">{{ g.name }}</div>
        <ConnectionItem
          v-for="c in g.items"
          :key="c.id"
          :conn="c"
          @connect="emit('connect', $event)"
          @edit="emit('edit', $event)"
          @remove="remove($event.id)"
          @favorite="toggleFavorite($event)"
          @context="openMenu"
        />
      </template>
    </div>

    <div v-else class="empty">
      <div class="empty-title">还没有保存机器</div>
      <p>新建一个连接后，它会出现在这里。</p>
    </div>

    <ContextMenu
      :open="menu.open"
      :x="menu.x"
      :y="menu.y"
      :items="menuItems"
      @select="onMenuSelect"
      @close="menu.open = false"
    />
  </aside>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--surface);
  border-right: 1px solid var(--line);
}
/* 主区列表页模式：去掉右边框，内容居中限宽。 */
.sidebar.home {
  background: var(--bg);
  border-right: 0;
}
.sidebar.home .search,
.sidebar.home .quick-actions,
.sidebar.home .new,
.sidebar.home .list {
  width: 100%;
  max-width: 720px;
  margin-inline: auto;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: var(--topbar-h);
  padding: 0 var(--sp-2) 0 var(--sp-4);
  border-bottom: 1px solid var(--line);
}
.brand {
  font-size: var(--fs-md);
  font-weight: 700;
  letter-spacing: 0.02em;
}
.head-actions {
  display: flex;
  gap: 2px;
}
.quick-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--sp-2);
  padding: var(--sp-3) var(--sp-3) 0;
}
.tool-btn {
  min-height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--sp-2);
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-sm);
  cursor: pointer;
}
.tool-btn:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.tool-icon {
  display: inline-grid;
  place-items: center;
  width: 18px;
  height: 18px;
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--accent-hover);
  line-height: 1;
}
.search {
  padding: var(--sp-3) var(--sp-3) var(--sp-2);
}
.search input {
  width: 100%;
  min-height: 36px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-sm);
}
.new {
  margin: 0 var(--sp-3) var(--sp-2);
  min-height: 38px;
  border: 1px solid var(--accent);
  border-radius: var(--radius);
  background: var(--accent);
  color: #fff;
  font-size: var(--fs-sm);
  cursor: pointer;
}
.new:hover {
  background: var(--accent-hover);
}
.list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 0 var(--sp-2) var(--sp-3);
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.group-title {
  padding: var(--sp-3) var(--sp-2) var(--sp-1);
  font-size: var(--fs-xs);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.empty {
  margin: var(--sp-4);
  padding: var(--sp-5);
  border: 1px dashed var(--line);
  border-radius: var(--radius);
  text-align: center;
  color: var(--muted);
}
.empty-title {
  margin-bottom: var(--sp-2);
  color: var(--text);
}
.empty p {
  margin: 0;
  font-size: var(--fs-sm);
}
.icon {
  width: 32px;
  height: 32px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  cursor: pointer;
}
.icon:hover {
  background: var(--surface-3);
  color: var(--text);
}
</style>
