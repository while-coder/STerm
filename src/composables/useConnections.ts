// 已保存连接：列表、搜索过滤、分组、收藏、增删改。模块级单例，包装 storage。
import { computed, ref } from "vue";
import type { SavedConnection } from "../api";
import {
  loadConnections,
  removeConnection,
  saveConnections,
  upsertConnection,
  type ConnectionsState,
  type Tombstones,
} from "../storage";

const UNGROUPED = "未分组";

export interface ConnectionGroup {
  name: string;
  items: SavedConnection[];
}

const connections = ref<SavedConnection[]>([]);
const tombstones = ref<Tombstones>({});
const query = ref("");
const initialized = ref(false);
const storageError = ref("");
// 用户主动增删改的计数器；云同步据此防抖触发推送。合并落盘不计入，避免回环。
const changeTick = ref(0);
let activeMasterPassword = "";

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

async function persist(password = activeMasterPassword) {
  if (!password) throw new Error("缺少主密码，无法保存机器列表");
  await saveConnections(connections.value, tombstones.value, password);
  storageError.value = "";
}

async function initConnections(masterPassword: string, resetOnFailure = false) {
  activeMasterPassword = masterPassword;
  try {
    const state = await loadConnections(masterPassword);
    connections.value = state.connections;
    tombstones.value = state.tombstones;
    storageError.value = "";
  } catch (e) {
    if (!resetOnFailure) {
      storageError.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
    connections.value = [];
    tombstones.value = {};
    storageError.value = "";
  }
  initialized.value = true;
  await persist(masterPassword);
}

async function reencryptConnections(masterPassword: string) {
  await persist(masterPassword);
  activeMasterPassword = masterPassword;
}

/** 用合并后的状态替换内存并落盘（供云同步调用）。 */
async function applyMerged(state: ConnectionsState) {
  connections.value = state.connections;
  tombstones.value = state.tombstones;
  await persist();
}

function resetConnections() {
  connections.value = [];
  tombstones.value = {};
  initialized.value = false;
  storageError.value = "";
  activeMasterPassword = "";
}

async function save(conn: SavedConnection) {
  const stamped = { ...conn, updatedAt: Date.now() };
  connections.value = upsertConnection(connections.value, stamped);
  // 复活：保存即清除可能存在的删除墓碑。
  if (tombstones.value[stamped.id] !== undefined) {
    const { [stamped.id]: _, ...rest } = tombstones.value;
    tombstones.value = rest;
  }
  await persist();
  changeTick.value++;
}

async function remove(id: string) {
  connections.value = removeConnection(connections.value, id);
  tombstones.value = { ...tombstones.value, [id]: Date.now() };
  await persist();
  changeTick.value++;
}

async function toggleFavorite(conn: SavedConnection) {
  await save({ ...conn, favorite: !conn.favorite });
}

export function useConnections() {
  return {
    connections,
    tombstones,
    changeTick,
    initialized,
    storageError,
    query,
    filtered,
    grouped,
    groupNames,
    initConnections,
    reencryptConnections,
    resetConnections,
    applyMerged,
    save,
    remove,
    toggleFavorite,
  };
}
