<script setup lang="ts">
// SFTP 文件浏览器：列目录、进入子目录、上传 / 下载。
import { ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { sftpHome, sftpList, sftpDownload, sftpUpload, type FileEntry } from "../api";

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

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    entries.value = await sftpList(props.id, cwd.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function listPath(path: string): Promise<boolean> {
  loading.value = true;
  error.value = "";
  try {
    const next = await sftpList(props.id, path);
    cwd.value = path;
    entries.value = next;
    emit("cwdChanged", cwd.value);
    return true;
  } catch (e) {
    error.value = String(e);
    return false;
  } finally {
    loading.value = false;
  }
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
  if (!entry.isDir) return;
  void listPath(entry.path);
}

function goUp() {
  void listPath(parentOf(cwd.value));
}

async function loadPath(path: string) {
  await listPath(path);
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

async function upload() {
  const local = await open({ multiple: false });
  if (!local || typeof local !== "string") return;
  try {
    await sftpUpload(props.id, local, joinRemote(cwd.value, basename(local)));
    await refresh();
  } catch (e) {
    error.value = String(e);
  }
}

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

// 连接就绪后可自动定位到家目录并加载。
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
    await loadPath(props.followPath);
  }
);
</script>

<template>
  <div class="fb">
    <div class="toolbar">
      <button @click="goUp" title="上级目录">⬆</button>
      <button @click="refresh" title="刷新">⟳</button>
      <button @click="upload" title="上传文件">⬆ 上传</button>
      <span class="cwd" :title="cwd">{{ cwd }}</span>
    </div>
    <div v-if="error" class="error">{{ error }}</div>
    <div class="list">
      <table>
        <tbody>
          <tr v-for="e in entries" :key="e.path" :class="{ dir: e.isDir }">
            <td class="name" @click="enter(e)">
              <span class="icon">{{ e.isDir ? "📁" : "📄" }}</span>{{ e.name }}
            </td>
            <td class="size">{{ e.isDir ? "" : fmtSize(e.size) }}</td>
            <td class="act">
              <button v-if="!e.isDir" @click="download(e)" title="下载">⬇</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="loading" class="hint">加载中…</div>
      <div v-else-if="!entries.length" class="hint">（空目录）</div>
    </div>
  </div>
</template>

<style scoped>
.fb {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--surface, #252526);
  color: var(--text, #d4d4d4);
  font-size: 13px;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px;
  border-bottom: 1px solid var(--line, #333);
}
.toolbar button {
  background: var(--surface-3, #333);
  color: var(--text, #d4d4d4);
  border: none;
  border-radius: 4px;
  padding: 3px 8px;
  cursor: pointer;
}
.toolbar button:hover {
  background: var(--surface-2, #444);
}
.cwd {
  margin-left: 6px;
  color: var(--muted, rgba(212, 212, 212, 0.8));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.list {
  flex: 1;
  overflow: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
}
td {
  padding: 3px 6px;
}
.name {
  cursor: pointer;
}
tr.dir .name {
  color: #4ec9b0;
}
.name:hover {
  background: var(--surface-2, #2a2d2e);
}
.icon {
  margin-right: 6px;
}
.size {
  text-align: right;
  color: var(--muted, rgba(212, 212, 212, 0.7));
  white-space: nowrap;
}
.act button {
  background: transparent;
  border: none;
  color: #569cd6;
  cursor: pointer;
}
.error {
  color: #f48771;
  padding: 6px;
}
.hint {
  padding: 10px;
  color: var(--muted, rgba(212, 212, 212, 0.6));
}
</style>
