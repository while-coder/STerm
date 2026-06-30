<script setup lang="ts">
// SFTP 文件浏览器：面包屑导航、列目录、进入子目录、新建文件夹 / 重命名 / 删除、上传 / 下载、拖拽上传。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { appDataDir, join } from "@tauri-apps/api/path";
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  sftpHome,
  sftpList,
  sftpDownload,
  sftpDownloadDir,
  sftpUpload,
  sftpMkdir,
  sftpCreateFile,
  sftpRename,
  sftpRemove,
  ensureDir,
  openLocalPath,
  readTextFile,
  writeTextFile,
  type FileEntry,
} from "../api";
import { useLongPress } from "../composables/useLongPress";
import { useTransfers } from "../composables/useTransfers";
import { useSettings } from "../composables/useSettings";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";

const { settings } = useSettings();

const props = defineProps<{
  id: string;
  sessionLabel?: string;
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

// 内置文本阅读器 / 编辑器：双击文本文件时展示其内容，可编辑并保存回远端。
// 非文本回退系统默认程序。
const viewer = ref<{
  name: string;
  remote: string; // 远端路径，保存时上传回去
  local: string; // 本地缓存路径
  original: string; // 原始内容，用于判断是否被修改
  content: string; // 当前（可能已编辑的）内容
} | null>(null);
const viewerSaving = ref(false);
const viewerDirty = computed(
  () => viewer.value !== null && viewer.value.content !== viewer.value.original
);
// 超过此大小不走内置阅读器，直接交给系统程序，避免把大文件读进内存卡界面。
const VIEWER_MAX_BYTES = 2 * 1024 * 1024;

const { transfers, enqueue, cancel: cancelTransfer, clearFinished } = useTransfers();
const showTransfers = computed({
  get: () => settings.transferListOpen,
  set: (v: boolean) => {
    settings.transferListOpen = v;
  },
});
const onlyCurrentSession = ref(true);
const activeCount = computed(
  () => transfers.value.filter((t) => t.status === "queued" || t.status === "running").length
);
const visibleTransfers = computed(() =>
  onlyCurrentSession.value
    ? transfers.value.filter((t) => t.sessionId === props.id)
    : transfers.value
);

// 注入当前会话信息后入队，便于按 session 过滤。
function startTransfer(opts: {
  kind: "download" | "upload";
  name: string;
  withProgress?: boolean;
  start: (transferId: string) => Promise<void>;
}) {
  return enqueue({ ...opts, sessionId: props.id, sessionLabel: props.sessionLabel ?? props.id });
}
function transferPercent(t: {
  transferred: number;
  total: number;
  status: string;
  progressDone?: boolean;
}): number {
  if (t.status === "done" || t.progressDone) return 100;
  if (!t.total) return 0;
  return Math.min(100, Math.round((t.transferred / t.total) * 100));
}

// —— 传输列表高度拖拽（拖顶边，向上变高）——
function startResizeTransfers(e: PointerEvent) {
  const startY = e.clientY;
  const startH = settings.transferListHeight;
  // 上限随容器高度动态计算，留出 56px 顶部边距，避免拖出屏幕。
  const panel = (e.currentTarget as HTMLElement).parentElement;
  const container = panel?.offsetParent as HTMLElement | null;
  const maxH = container ? Math.max(120, container.clientHeight - 140) : 520;
  const move = (ev: PointerEvent) => {
    const h = startH + (startY - ev.clientY);
    settings.transferListHeight = Math.min(maxH, Math.max(120, h));
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

// 右键 / 长按上下文菜单。entry 为空时是空白区菜单（上传 / 新建文件夹）。
const lp = useLongPress();
const lpBlank = useLongPress();
const menu = ref<{ open: boolean; x: number; y: number; entry: FileEntry | null }>({
  open: false,
  x: 0,
  y: 0,
  entry: null,
});
// 菜单作用对象：右键项若在多选区内，则作用于整个选区，否则仅该项。
const menuTargets = computed<FileEntry[]>(() => {
  const e = menu.value.entry;
  if (!e) return [];
  if (selected.value.has(e.path) && selected.value.size > 1) {
    return entries.value.filter((x) => selected.value.has(x.path));
  }
  return [e];
});
const menuItems = computed<MenuItem[]>(() => {
  const e = menu.value.entry;
  if (!e) {
    return [
      { key: "upload", label: "上传文件" },
      { key: "newfile", label: "新建文件" },
      { key: "mkdir", label: "新建文件夹" },
    ];
  }
  const n = menuTargets.value.length;
  if (n > 1) {
    return [
      { key: "download", label: `下载 ${n} 项` },
      { key: "delete", label: `删除 ${n} 项`, danger: true },
    ];
  }
  return [
    { key: "download", label: e.isDir ? "下载文件夹" : "下载" },
    { key: "rename", label: "重命名" },
    { key: "delete", label: "删除", danger: true },
  ];
});
function openMenuAt(e: MouseEvent | PointerEvent, entry: FileEntry) {
  menu.value = { open: true, x: e.clientX, y: e.clientY, entry };
}
function openBlankMenu(e: MouseEvent | PointerEvent) {
  menu.value = { open: true, x: e.clientX, y: e.clientY, entry: null };
}
function onMenuSelect(key: string) {
  if (key === "upload") return void pickUpload();
  if (key === "newfile") return openNewFile();
  if (key === "mkdir") return openMkdir();
  const targets = menuTargets.value;
  if (!targets.length) return;
  if (targets.length > 1) {
    if (key === "download") void downloadItems(targets);
    else if (key === "delete") askDeleteItems(targets);
    return;
  }
  const entry = targets[0];
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
    if (hit && entry) next.add(entry.path);
  });
  selected.value = next;
}

function joinLocal(dir: string, name: string): string {
  const sep = dir.includes("\\") ? "\\" : "/";
  return dir.endsWith(sep) ? `${dir}${name}` : `${dir}${sep}${name}`;
}

function knownFileSize(entry: FileEntry): number | undefined {
  return entry.size > 0 ? entry.size : undefined;
}

async function downloadItems(items: FileEntry[]) {
  if (!items.length) return;
  const dir = await open({ directory: true });
  if (!dir || typeof dir !== "string") return;
  for (const it of items) {
    const target = joinLocal(dir, it.name);
    startTransfer({
      kind: "download",
      name: it.name,
      withProgress: true,
      start: (tid) =>
        it.isDir
          ? sftpDownloadDir(props.id, it.path, target, tid)
          : sftpDownload(props.id, it.path, target, tid, knownFileSize(it)),
    });
  }
  clearSelection();
}
function downloadSelected() {
  void downloadItems(entries.value.filter((e) => selected.value.has(e.path)));
}

function askDeleteItems(items: FileEntry[]) {
  if (!items.length) return;
  ask.value = { mode: "delete", title: `删除选中的 ${items.length} 项？`, value: "", entries: items };
}
function deleteSelected() {
  askDeleteItems(entries.value.filter((e) => selected.value.has(e.path)));
}

// 输入对话框（新建文件夹 / 重命名 / 删除确认）。
type Ask = {
  mode: "mkdir" | "newfile" | "rename" | "delete";
  title: string;
  value: string;
  entry?: FileEntry;
  entries?: FileEntry[];
};
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

// 双击查看：下载到本地缓存目录后用系统默认程序打开。
async function cacheBaseDir(): Promise<string> {
  const custom = settings.sftpCacheDir?.trim();
  if (custom) return custom;
  return join(await appDataDir(), "viewer-cache");
}
async function openFile(entry: FileEntry) {
  if (entry.isDir) return;
  try {
    const base = await cacheBaseDir();
    await ensureDir(base);
    const local = await join(base, entry.name);
    startTransfer({
      kind: "download",
      name: entry.name,
      withProgress: true,
      start: async (tid) => {
        await sftpDownload(props.id, entry.path, local, tid, knownFileSize(entry));
        await openDownloaded(entry, local);
      },
    });
  } catch (e) {
    error.value = String(e);
  }
}

// 下载完成后决定如何打开：能按 UTF-8 文本读取且不含空字节 → 内置阅读器；
// 否则（二进制 / 非 UTF-8 / 过大 / 读取失败）回退系统默认程序。
async function openDownloaded(entry: FileEntry, local: string) {
  const size = knownFileSize(entry);
  if (size === undefined || size <= VIEWER_MAX_BYTES) {
    try {
      const content = await readTextFile(local);
      if (content.indexOf(String.fromCharCode(0)) === -1) {
        viewer.value = {
          name: entry.name,
          remote: entry.path,
          local,
          original: content,
          content,
        };
        return;
      }
    } catch {
      /* 非文本或读取失败，走系统打开 */
    }
  }
  void openLocalPath(local).catch((e) => {
    error.value = `打开本地文件失败：${String(e)}`;
  });
}

async function copyViewerContent() {
  if (!viewer.value) return;
  try {
    await navigator.clipboard.writeText(viewer.value.content);
  } catch {
    /* 忽略复制失败 */
  }
}

// 保存：先写本地缓存，再上传回远端，成功后把 original 同步为当前内容（清除“已修改”标记）。
async function saveViewer() {
  const v = viewer.value;
  if (!v || viewerSaving.value || v.content === v.original) return;
  viewerSaving.value = true;
  try {
    await writeTextFile(v.local, v.content);
    await sftpUpload(props.id, v.local, v.remote, crypto.randomUUID());
    v.original = v.content;
    refresh();
  } catch (e) {
    error.value = `保存失败：${String(e)}`;
  } finally {
    viewerSaving.value = false;
  }
}

// 关闭阅读器；有未保存修改时先确认，避免误丢编辑。
function closeViewer() {
  if (viewerDirty.value && !window.confirm("有未保存的修改，确定关闭吗？")) return;
  viewer.value = null;
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
  startTransfer({
    kind: "download",
    name: entry.name,
    withProgress: true,
    start: (tid) => sftpDownload(props.id, entry.path, target, tid, knownFileSize(entry)),
  });
}

// 下载目录：选目标父文件夹，在其下重建该目录。
async function downloadDir(entry: FileEntry) {
  const dir = await open({ directory: true });
  if (!dir || typeof dir !== "string") return;
  const target = joinLocal(dir, entry.name);
  startTransfer({
    kind: "download",
    name: entry.name,
    withProgress: true,
    start: (tid) => sftpDownloadDir(props.id, entry.path, target, tid),
  });
}

async function uploadFrom(localPaths: string[]) {
  for (const local of localPaths) {
    const name = basename(local);
    const remote = joinRemote(cwd.value, name); // 入队时固定目标，避免随后切目录跑偏
    startTransfer({
      kind: "upload",
      name,
      withProgress: true,
      start: async (tid) => {
        await sftpUpload(props.id, local, remote, tid);
        refresh();
      },
    });
  }
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
function openNewFile() {
  ask.value = { mode: "newfile", title: "新建文件", value: "" };
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
    } else if (a.mode === "newfile") {
      const name = a.value.trim();
      if (!name) return;
      await sftpCreateFile(props.id, joinRemote(cwd.value, name));
    } else if (a.mode === "rename" && a.entry) {
      const name = a.value.trim();
      if (!name || name === a.entry.name) {
        ask.value = null;
        return;
      }
      await sftpRename(props.id, a.entry.path, joinRemote(parentOf(a.entry.path), name));
    } else if (a.mode === "delete") {
      // 批量删除：逐项删除，单项失败不中断其余，最后汇总错误。
      const items = a.entries ?? (a.entry ? [a.entry] : []);
      const errs: string[] = [];
      for (const it of items) {
        try {
          await sftpRemove(props.id, it.path, it.isDir);
        } catch (e) {
          errs.push(`${it.name}: ${e}`);
        }
      }
      clearSelection();
      ask.value = null;
      refresh();
      if (errs.length) error.value = errs.join("；");
      return;
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
  window.addEventListener("keydown", onGlobalKeydown);
});
onBeforeUnmount(() => {
  unlistenDrop?.();
  window.removeEventListener("keydown", onGlobalKeydown);
});

// 阅读器打开时按 Esc 关闭（有未保存修改会经 closeViewer 二次确认）。
function onGlobalKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && viewer.value) {
    e.preventDefault();
    closeViewer();
  }
}

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
      <button class="tb" type="button" title="新建文件" @click="openNewFile">＋📄</button>
      <button class="tb" type="button" title="新建文件夹" @click="openMkdir">＋📁</button>
      <button class="tb" type="button" title="上传文件" @click="pickUpload">⬆ 上传</button>
      <button
        class="tb transfers-btn"
        type="button"
        title="传输列表"
        @click="showTransfers = !showTransfers"
      >
        ⇅ 传输<span v-if="activeCount" class="badge">{{ activeCount }}</span>
      </button>
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
      <button type="button" class="danger" @click="deleteSelected">删除</button>
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
          @dblclick="openFile(e)"
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

    <div v-if="showTransfers" class="transfers" :style="{ height: settings.transferListHeight + 'px' }">
      <div
        class="transfers-resizer"
        title="拖拽调整传输列表高度"
        @pointerdown.prevent="startResizeTransfers"
      ></div>
      <div class="transfers-head">
        <span>传输列表</span>
        <label class="scope">
          <input v-model="onlyCurrentSession" type="checkbox" />
          仅当前会话
        </label>
        <button type="button" @click="clearFinished">清除已完成</button>
        <button type="button" @click="showTransfers = false">×</button>
      </div>
      <div class="transfers-body">
        <div v-if="!visibleTransfers.length" class="hint">暂无传输</div>
        <div v-for="t in visibleTransfers" :key="t.id" class="tr" :class="t.status">
          <div class="tr-top">
            <span class="tr-name" :title="t.name">
              {{ t.kind === "upload" ? "⬆" : "⬇" }} {{ t.name }}
              <em v-if="!onlyCurrentSession" class="tr-session">· {{ t.sessionLabel }}</em>
            </span>
            <span class="tr-meta">
              <template v-if="t.status === 'error'">失败</template>
              <template v-else-if="t.status === 'cancelled'">已取消</template>
              <template v-else-if="t.status === 'done'">完成</template>
              <template v-else-if="t.status === 'queued'">排队中</template>
              <template v-else>{{ transferPercent(t) }}%</template>
            </span>
            <button
              v-if="t.status === 'queued' || t.status === 'running'"
              class="tr-cancel"
              type="button"
              title="取消"
              @click="cancelTransfer(t.id)"
            >
              ×
            </button>
          </div>
          <div class="tr-bar">
            <div
              class="tr-fill"
              :class="{ indeterminate: t.status === 'running' && t.kind === 'upload' }"
              :style="{ width: transferPercent(t) + '%' }"
            ></div>
          </div>
          <div v-if="t.error" class="tr-err" :title="t.error">{{ t.error }}</div>
        </div>
      </div>
    </div>

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

    <Teleport to="body">
      <div v-if="viewer" class="viewer-backdrop" @click.self="closeViewer">
        <div class="viewer">
          <div class="viewer-head">
            <span class="viewer-name" :title="viewer.remote">
              📄 {{ viewer.name }}
              <em v-if="viewerDirty" class="viewer-dirty">· 未保存</em>
            </span>
            <button type="button" @click="copyViewerContent">复制</button>
            <button
              type="button"
              class="primary"
              :disabled="!viewerDirty || viewerSaving"
              @click="saveViewer"
            >
              {{ viewerSaving ? "保存中…" : "保存" }}
            </button>
            <button type="button" @click="closeViewer">关闭</button>
          </div>
          <textarea
            v-model="viewer.content"
            class="viewer-body"
            spellcheck="false"
            @keydown.ctrl.s.prevent="saveViewer"
            @keydown.meta.s.prevent="saveViewer"
          ></textarea>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.fb {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
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
  white-space: nowrap;
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
.selbar button.danger {
  border-color: var(--danger);
  background: var(--danger);
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
.transfers-btn {
  position: relative;
  margin-left: auto;
}
.badge {
  margin-left: 4px;
  padding: 0 5px;
  border-radius: 8px;
  background: var(--accent);
  color: #fff;
  font-size: var(--fs-xs);
}
.transfers {
  flex: 0 0 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-top: 1px solid var(--line);
  background: var(--bg);
}
.transfers-resizer {
  flex: 0 0 auto;
  height: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: row-resize;
  touch-action: none;
  background: var(--surface);
  border-bottom: 1px solid var(--line);
}
.transfers-resizer::before {
  content: "";
  width: 36px;
  height: 3px;
  border-radius: 3px;
  background: var(--line);
}
.transfers-resizer:hover::before {
  background: var(--muted);
}
.transfers-head {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-3);
  border-bottom: 1px solid var(--line);
  font-size: var(--fs-sm);
}
.transfers-head > span {
  flex: 1;
  font-weight: 600;
  white-space: nowrap;
}
.scope {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--muted);
  font-weight: 400;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}
.tr-session {
  font-style: normal;
  color: var(--muted);
  font-size: var(--fs-xs);
}
.transfers-head button {
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  padding: 2px 8px;
  cursor: pointer;
  white-space: nowrap;
  flex-shrink: 0;
}
.transfers-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: var(--sp-2);
}
.tr {
  padding: var(--sp-2) 0;
}
.tr + .tr {
  border-top: 1px solid var(--line);
}
.tr-top {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: var(--fs-sm);
}
.tr-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tr-meta {
  flex: 0 0 auto;
  color: var(--muted);
  font-size: var(--fs-xs);
}
.tr-bar {
  margin-top: 4px;
  height: 5px;
  border-radius: 3px;
  background: var(--surface-3);
  overflow: hidden;
}
.tr-fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.15s;
}
.tr.done .tr-fill {
  background: var(--ok);
}
.tr.error .tr-fill {
  background: var(--danger);
  width: 100% !important;
}
.tr.cancelled .tr-fill {
  background: var(--muted);
}
.tr-cancel {
  flex: 0 0 auto;
  width: 20px;
  height: 20px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--muted);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
}
.tr-cancel:hover {
  background: var(--surface-3);
  color: var(--danger);
}
.tr-fill.indeterminate {
  width: 40% !important;
  animation: tr-indet 1.1s ease-in-out infinite;
}
@keyframes tr-indet {
  0% { margin-left: -40%; }
  100% { margin-left: 100%; }
}
.tr-err {
  margin-top: 2px;
  color: var(--danger);
  font-size: var(--fs-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

/* —— 内置文本阅读器 / 编辑器 —— */
.viewer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: stretch;
  justify-content: center;
  padding: var(--sp-3);
  background: var(--overlay);
}
.viewer {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--bg);
  box-shadow: var(--shadow);
  overflow: hidden;
}
.viewer-head {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  padding: var(--sp-2) var(--sp-3);
  border-bottom: 1px solid var(--line);
  font-size: var(--fs-sm);
}
.viewer-name {
  flex: 1;
  min-width: 0;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.viewer-dirty {
  font-style: normal;
  font-weight: 400;
  color: var(--accent-hover);
}
.viewer-head button {
  min-height: 30px;
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  white-space: nowrap;
  flex-shrink: 0;
  cursor: pointer;
}
.viewer-head .primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}
.viewer-head button:disabled {
  opacity: 0.5;
  cursor: default;
}
.viewer-body {
  flex: 1;
  min-height: 0;
  width: 100%;
  padding: var(--sp-3);
  border: 0;
  resize: none;
  background: var(--surface);
  color: var(--text);
  font-family: var(--font-mono, monospace);
  font-size: var(--fs-sm);
  line-height: 1.5;
  white-space: pre;
  overflow: auto;
  tab-size: 4;
}
.viewer-body:focus {
  outline: none;
}
</style>
