<script setup lang="ts">
// SFTP 文件浏览器：面包屑导航、列目录、进入子目录、新建文件夹 / 重命名 / 删除、上传 / 下载、拖拽上传。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  sftpHome,
  sftpList,
  sftpDownload,
  sftpDownloadDir,
  sftpUpload,
  sftpMkdir,
  sftpRename,
  sftpRemove,
  type FileEntry,
} from "../api";
import { useLongPress } from "../composables/useLongPress";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";

const props = defineProps<{
  id: string;
  connected: boolean;
  autoHome: boolean;
  followPath?: string;
  followToken?: number;
}>();
const emit = defineEmits<{
  cwdChanged: [path: string];
  homeChanged: [path: string];
}>();

const cwd = ref("/");
const entries = ref<FileEntry[]>([]);
const loading = ref(false);
const error = ref("");
const homeLoaded = ref(false);
const dragOver = ref(false);

// 右键 / 长按上下文菜单。entry 为空时是空白区菜单（上传 / 新建文件夹）。
const lp = useLongPress();
const lpBlank = useLongPress();
const menu = ref<{ open: boolean; x: number; y: number; entry: FileEntry | null }>({
  open: false,
  x: 0,
  y: 0,
  entry: null,
});
const menuItems = computed<MenuItem[]>(() => {
  const e = menu.value.entry;
  if (!e) {
    return [
      { key: "upload", label: "上传文件" },
      { key: "mkdir", label: "新建文件夹" },
    ];
  }
  const items: MenuItem[] = [];
  items.push({ key: "download", label: e.isDir ? "下载文件夹" : "下载" });
  items.push({ key: "rename", label: "重命名" });
  items.push({ key: "delete", label: "删除", danger: true });
  return items;
});
function openMenuAt(e: MouseEvent | PointerEvent, entry: FileEntry) {
  menu.value = { open: true, x: e.clientX, y: e.clientY, entry };
}
function openBlankMenu(e: MouseEvent | PointerEvent) {
  menu.value = { open: true, x: e.clientX, y: e.clientY, entry: null };
}
function onMenuSelect(key: string) {
  if (key === "upload") return void pickUpload();
  if (key === "mkdir") return openMkdir();
  const entry = menu.value.entry;
  if (!entry) return;
  if (key === "download") void (entry.isDir ? downloadDir(entry) : download(entry));
  else if (key === "rename") openRename(entry);
  else if (key === "delete") openDelete(entry);
}
function onNameClick(entry: FileEntry, ev: MouseEvent) {
  if (lp.suppressed.value) return;
  if (ev.ctrlKey || ev.metaKey) {
    toggleSelect(entry);
    return;
  }
  enter(entry);
}

// —— 多选 / 框选下载 ——
const listEl = ref<HTMLElement | null>(null);
const selected = ref<Set<string>>(new Set());
const marquee = ref({ active: false, left: 0, top: 0, width: 0, height: 0 });
let marqueeStart: { x: number; y: number } | null = null;

function toggleSelect(entry: FileEntry) {
  if (entry.isDir) return; // 目录不参与下载选择
  const next = new Set(selected.value);
  next.has(entry.path) ? next.delete(entry.path) : next.add(entry.path);
  selected.value = next;
}
function clearSelection() {
  selected.value = new Set();
}

function onListPointerDown(e: PointerEvent) {
  if (e.target !== e.currentTarget) return; // 仅空白区域
  if (e.pointerType === "mouse") {
    if (e.button !== 0) return;
    marqueeStart = { x: e.clientX, y: e.clientY }; // 移动超过阈值才视为框选
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  } else {
    lpBlank.start(e, openBlankMenu);
  }
}
function onListPointerMove(e: PointerEvent) {
  lpBlank.move(e);
  if (!marqueeStart) return;
  if (!marquee.value.active && Math.hypot(e.clientX - marqueeStart.x, e.clientY - marqueeStart.y) < 5)
    return;
  const left = Math.min(marqueeStart.x, e.clientX);
  const top = Math.min(marqueeStart.y, e.clientY);
  const right = Math.max(marqueeStart.x, e.clientX);
  const bottom = Math.max(marqueeStart.y, e.clientY);
  marquee.value = { active: true, left, top, width: right - left, height: bottom - top };
  applyMarquee(left, top, right, bottom);
}
function onListPointerUp() {
  lpBlank.end();
  if (marqueeStart && !marquee.value.active) clearSelection(); // 空白单击 = 清除
  marqueeStart = null;
  marquee.value.active = false;
}
function applyMarquee(left: number, top: number, right: number, bottom: number) {
  const rows = listEl.value?.querySelectorAll<HTMLElement>(".row");
  if (!rows) return;
  const next = new Set<string>();
  rows.forEach((el, i) => {
    const r = el.getBoundingClientRect();
    const hit = !(r.right < left || r.left > right || r.bottom < top || r.top > bottom);
    const entry = entries.value[i];
    if (hit && entry && !entry.isDir) next.add(entry.path);
  });
  selected.value = next;
}

function joinLocal(dir: string, name: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
}
async function downloadSelected() {
  const files = entries.value.filter((e) => !e.isDir && selected.value.has(e.path));
  if (!files.length) return;
  const dir = await open({ directory: true });
  if (!dir || typeof dir !== "string") return;
  error.value = "";
  for (const f of files) {
    try {
      await sftpDownload(props.id, f.path, joinLocal(dir, f.name));
    } catch (e) {
      error.value = String(e);
    }
  }
  clearSelection();
}

// 输入对话框（新建文件夹 / 重命名 / 删除确认）。
type Ask = { mode: "mkdir" | "rename" | "delete"; title: string; value: string; entry?: FileEntry };
const ask = ref<Ask | null>(null);

const crumbs = computed(() => {
  const parts = cwd.value.split("/").filter(Boolean);
  const acc: { name: string; path: string }[] = [{ name: "/", path: "/" }];
  let cur = "";
  for (const p of parts) {
    cur += `/${p}`;
    acc.push({ name: p, path: cur });
  }
  return acc;
});

async function listPath(path: string): Promise<boolean> {
  loading.value = true;
  error.value = "";
  try {
    const next = await sftpList(props.id, path);
    cwd.value = path;
    entries.value = next;
    selected.value = new Set(); // 切换目录后清空选择
    emit("cwdChanged", cwd.value);
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  } finally {
    loading.value = false;
  }
}

function refresh() {
  void listPath(cwd.value);
}

function parentOf(path: string): string {
  const p = path.replace(/\/+$/, "");
  const i = p.lastIndexOf("/");
  return i <= 0 ? "/" : p.slice(0, i);
}

function basename(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

function joinRemote(dir: string, name: string): string {
  return dir.endsWith("/") ? `${dir}${name}` : `${dir}/${name}`;
}

function enter(entry: FileEntry) {
  if (entry.isDir) void listPath(entry.path);
}

function goUp() {
  void listPath(parentOf(cwd.value));
}

async function loadHome() {
  let home = "/";
  try {
    home = await sftpHome(props.id);
  } catch {
    home = "/";
  }
  homeLoaded.value = true;
  emit("homeChanged", home);
  await listPath(home);
}

async function download(entry: FileEntry) {
  const target = await save({ defaultPath: entry.name });
  if (!target) return;
  try {
    await sftpDownload(props.id, entry.path, target);
  } catch (e) {
    error.value = String(e);
  }
}

async function uploadFrom(localPaths: string[]) {
  error.value = "";
  for (const local of localPaths) {
    try {
      await sftpUpload(props.id, local, joinRemote(cwd.value, basename(local)));
    } catch (e) {
      error.value = String(e);
    }
  }
  refresh();
}

async function pickUpload() {
  const local = await open({ multiple: true });
  if (!local) return;
  await uploadFrom(Array.isArray(local) ? local : [local]);
}

// —— 输入对话框 ——
function openMkdir() {
  ask.value = { mode: "mkdir", title: "新建文件夹", value: "" };
}
function openRename(entry: FileEntry) {
  ask.value = { mode: "rename", title: "重命名", value: entry.name, entry };
}
function openDelete(entry: FileEntry) {
  ask.value = { mode: "delete", title: `删除 ${entry.name}？`, value: "", entry };
}

async function confirmAsk() {
  const a = ask.value;
  if (!a) return;
  try {
    if (a.mode === "mkdir") {
      const name = a.value.trim();
      if (!name) return;
      await sftpMkdir(props.id, joinRemote(cwd.value, name));
    } else if (a.mode === "rename" && a.entry) {
      const name = a.value.trim();
      if (!name || name === a.entry.name) {
        ask.value = null;
        return;
      }
      await sftpRename(props.id, a.entry.path, joinRemote(parentOf(a.entry.path), name));
    } else if (a.mode === "delete" && a.entry) {
      await sftpRemove(props.id, a.entry.path, a.entry.isDir);
    }
    ask.value = null;
    refresh();
  } catch (e) {
    error.value = String(e);
    ask.value = null;
  }
}

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

// —— Tauri webview 原生文件拖拽（携带本地路径）——
let unlistenDrop: UnlistenFn | null = null;
onMounted(async () => {
  try {
    unlistenDrop = await getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "over" || p.type === "enter") {
        dragOver.value = true;
      } else if (p.type === "drop") {
        dragOver.value = false;
        if (props.connected && p.paths?.length) void uploadFrom(p.paths);
      } else {
        dragOver.value = false;
      }
    });
  } catch {
    /* 移动端 / 不支持时静默 */
  }
});
onBeforeUnmount(() => unlistenDrop?.());

watch(
  () => props.connected,
  async (connected) => {
    if (!connected || !props.autoHome || homeLoaded.value) return;
    await loadHome();
  },
  { immediate: true }
);

watch(
  () => props.autoHome,
  async (autoHome) => {
    if (!props.connected || !autoHome || homeLoaded.value) return;
    await loadHome();
  }
);

watch(
  () => props.followToken,
  async () => {
    if (!props.connected || !props.followPath) return;
    await listPath(props.followPath);
  }
);
</script>

<template>
  <div class="fb" :class="{ 'drag-over': dragOver }">
    <div class="toolbar">
      <button class="tb" type="button" title="上级目录" @click="goUp">⬆</button>
      <button class="tb" type="button" title="刷新" @click="refresh">⟳</button>
      <button class="tb" type="button" title="新建文件夹" @click="openMkdir">＋📁</button>
      <button class="tb" type="button" title="上传文件" @click="pickUpload">⬆ 上传</button>
    </div>

    <nav class="crumbs">
      <button
        v-for="(c, i) in crumbs"
        :key="c.path"
        class="crumb"
        type="button"
        @click="listPath(c.path)"
      >
        <span v-if="i > 0" class="sep">/</span>{{ c.name }}
      </button>
    </nav>

    <div v-if="error" class="error">{{ error }}</div>

    <div v-if="selected.size" class="selbar">
      <span>已选 {{ selected.size }} 项</span>
      <button type="button" class="primary" @click="downloadSelected">下载</button>
      <button type="button" @click="clearSelection">清除</button>
    </div>

    <div
      ref="listEl"
      class="list"
      @contextmenu.prevent="openBlankMenu"
      @pointerdown="onListPointerDown"
      @pointermove="onListPointerMove"
      @pointerup="onListPointerUp"
      @pointercancel="onListPointerUp"
    >
      <div
        v-for="e in entries"
        :key="e.path"
        class="row"
        :class="{ dir: e.isDir, selected: selected.has(e.path) }"
        @contextmenu.prevent.stop="openMenuAt($event, e)"
      >
        <button
          class="name"
          type="button"
          @click="onNameClick(e, $event)"
          @pointerdown="lp.start($event, (ev) => openMenuAt(ev, e))"
          @pointermove="lp.move"
          @pointerup="lp.end"
          @pointercancel="lp.end"
        >
          <span class="icon">{{ e.isDir ? "📁" : "📄" }}</span>
          <span class="label">{{ e.name }}</span>
        </button>
        <span class="size">{{ e.isDir ? "" : fmtSize(e.size) }}</span>
        <button class="rb" type="button" title="操作" @click="openMenuAt($event, e)">⋯</button>
      </div>
      <div v-if="loading" class="hint">加载中…</div>
      <div v-else-if="!entries.length" class="hint">（空目录）</div>
    </div>

    <div v-if="dragOver" class="drop-mask">松开以上传到 {{ cwd }}</div>

    <div
      v-if="marquee.active"
      class="marquee"
      :style="{
        left: marquee.left + 'px',
        top: marquee.top + 'px',
        width: marquee.width + 'px',
        height: marquee.height + 'px',
      }"
    ></div>

    <ContextMenu
      :open="menu.open"
      :x="menu.x"
      :y="menu.y"
      :items="menuItems"
      @select="onMenuSelect"
      @close="menu.open = false"
    />

    <div v-if="ask" class="ask-backdrop" @click.self="ask = null">
      <div class="ask">
        <div class="ask-title">{{ ask.title }}</div>
        <input
          v-if="ask.mode !== 'delete'"
          v-model="ask.value"
          placeholder="名称"
          @keydown.enter="confirmAsk"
        />
        <div class="ask-actions">
          <button type="button" @click="ask = null">取消</button>
          <button
            type="button"
            class="primary"
            :class="{ danger: ask.mode === 'delete' }"
            @click="confirmAsk"
          >
            {{ ask.mode === "delete" ? "删除" : "确定" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fb {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  background: var(--surface);
  color: var(--text);
  font-size: var(--fs-sm);
}
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--sp-1);
  padding: var(--sp-2);
  border-bottom: 1px solid var(--line);
}
.tb {
  min-height: 30px;
  padding: 0 var(--sp-2);
  border: 0;
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.tb:hover {
  background: var(--surface-2);
}
.crumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  padding: var(--sp-1) var(--sp-2);
  border-bottom: 1px solid var(--line);
  overflow-x: auto;
}
.crumb {
  border: 0;
  padding: 2px;
  background: transparent;
  color: var(--muted);
  font-size: var(--fs-xs);
  cursor: pointer;
  white-space: nowrap;
}
.crumb:hover {
  color: var(--text);
}
.crumb:last-child {
  color: var(--text);
  font-weight: 600;
}
.sep {
  margin: 0 2px;
  color: var(--muted);
}
.list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  user-select: none;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: 0 var(--sp-2);
  min-height: 34px;
}
.row:hover {
  background: var(--surface-2);
}
.row.selected {
  background: var(--accent-soft);
}
.selbar {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-1) var(--sp-2);
  border-bottom: 1px solid var(--line);
  background: var(--surface-2);
  font-size: var(--fs-xs);
}
.selbar span {
  flex: 1;
  color: var(--muted);
}
.selbar button {
  min-height: 28px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.selbar button.primary {
  border-color: var(--accent);
  background: var(--accent);
  color: #fff;
}
.marquee {
  position: fixed;
  z-index: 60;
  border: 1px solid var(--accent);
  background: var(--accent-soft);
  pointer-events: none;
}
.name {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  flex: 1;
  min-width: 0;
  border: 0;
  padding: var(--sp-1) 0;
  background: transparent;
  color: var(--text);
  text-align: left;
  cursor: pointer;
}
.row.dir .name .label {
  color: #4ec9b0;
}
.label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.icon {
  flex: 0 0 auto;
}
.size {
  flex: 0 0 auto;
  color: var(--muted);
  white-space: nowrap;
  font-size: var(--fs-xs);
}
.rb {
  flex: 0 0 auto;
  width: 28px;
  height: 28px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  cursor: pointer;
}
.rb:hover {
  background: var(--surface-3);
  color: var(--text);
}
.hint {
  padding: var(--sp-3);
  color: var(--muted);
}
.error {
  padding: var(--sp-2);
  color: var(--warn);
}
.fb.drag-over {
  outline: 2px dashed var(--accent);
  outline-offset: -2px;
}
.drop-mask {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent-soft);
  color: var(--text);
  pointer-events: none;
}
.ask-backdrop {
  position: absolute;
  inset: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--sp-4);
  background: var(--overlay);
}
.ask {
  width: 100%;
  max-width: 280px;
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
  padding: var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--bg);
  box-shadow: var(--shadow);
}
.ask-title {
  font-weight: 600;
}
.ask input {
  min-height: 36px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--text);
}
.ask-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-2);
}
.ask-actions button {
  min-height: 34px;
  padding: 0 var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
.ask-actions .primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.ask-actions .primary.danger {
  background: var(--danger);
  border-color: var(--danger);
}
</style>
