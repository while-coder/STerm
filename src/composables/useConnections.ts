// 已保存连接：列表、搜索过滤、分组、收藏、增删改。模块级单例，包装 storage。
import { computed, ref } from "vue";
import type { SavedConnection } from "../api";
import { loadConnections, removeConnection, upsertConnection } from "../storage";

const UNGROUPED = "未分组";

export interface ConnectionGroup {
  name: string;
  items: SavedConnection[];
}

const connections = ref<SavedConnection[]>(loadConnections());
const query = ref("");

/** 按搜索词过滤（匹配 label / host / username / 分组）。 */
const filtered = computed<SavedConnection[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return connections.value;
  return connections.value.filter((c) =>
    [c.label, c.host, c.username, c.group ?? ""]
      .some((field) => field.toLowerCase().includes(q))
  );
});

/** 收藏置顶，其余按分组聚合；分组内按 label 排序。 */
const grouped = computed<{ favorites: SavedConnection[]; groups: ConnectionGroup[] }>(() => {
  const list = filtered.value;
  const favorites = list.filter((c) => c.favorite);
  const byGroup = new Map<string, SavedConnection[]>();
  for (const c of list) {
    const key = c.group?.trim() || UNGROUPED;
    const arr = byGroup.get(key) ?? [];
    arr.push(c);
    byGroup.set(key, arr);
  }
  const groups: ConnectionGroup[] = [...byGroup.entries()]
    .sort(([a], [b]) => {
      if (a === UNGROUPED) return 1;
      if (b === UNGROUPED) return -1;
      return a.localeCompare(b);
    })
    .map(([name, items]) => ({
      name,
      items: [...items].sort((x, y) => x.label.localeCompare(y.label)),
    }));
  return { favorites, groups };
});

/** 已有分组名集合（供表单下拉建议）。 */
const groupNames = computed<string[]>(() => {
  const set = new Set<string>();
  for (const c of connections.value) {
    const g = c.group?.trim();
    if (g) set.add(g);
  }
  return [...set].sort((a, b) => a.localeCompare(b));
});

function save(conn: SavedConnection) {
  connections.value = upsertConnection(connections.value, conn);
}

function remove(id: string) {
  connections.value = removeConnection(connections.value, id);
}

function toggleFavorite(conn: SavedConnection) {
  save({ ...conn, favorite: !conn.favorite });
}

export function useConnections() {
  return {
    connections,
    query,
    filtered,
    grouped,
    groupNames,
    save,
    remove,
    toggleFavorite,
  };
}
