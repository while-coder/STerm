<script setup lang="ts">
// xterm.js 终端：挂载后建立连接，桥接后端 PTY 数据流与键盘输入。
import { nextTick, onMounted, onBeforeUnmount, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { sshConnect, sshWrite, sshResize, sshDisconnect, type ConnectOpts } from "../api";
import { useResponsive } from "../composables/useResponsive";

type TerminalThemeMode = "dark" | "light";

const props = defineProps<{
  opts: Omit<ConnectOpts, "cols" | "rows">;
  active?: boolean;
  theme?: TerminalThemeMode;
}>();
const emit = defineEmits<{
  connected: [];
  error: [msg: string];
  closed: [];
  cwd: [path: string];
}>();

const { isMobile } = useResponsive();
const host = ref<HTMLDivElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
const unlisten: UnlistenFn[] = [];
const ctrlArmed = ref(false);
let disposeOsc7: (() => void) | null = null;
// OSC 7 上报去重：仅当真实 cwd 变化时才通知上层，避免每个提示符都触发跟随。
let lastCwd = "";

function terminalTheme(mode: TerminalThemeMode = "dark") {
  return mode === "light"
    ? { background: "#ffffff", foreground: "#1f2328", cursor: "#0b5cad" }
    : { background: "#1e1e1e", foreground: "#d4d4d4", cursor: "#d4d4d4" };
}

/** base64 -> 字节数组，保证二进制流（含多字节 UTF-8）完整还原。 */
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

/**
 * 解析 OSC 7 序列的负载 `file://host/path`，返回其中的绝对路径（URL 解码）。
 * 无法识别时返回 null。
 */
function parseOsc7(data: string): string | null {
  if (!data.startsWith("file://")) return null;
  const rest = data.slice("file://".length);
  const slash = rest.indexOf("/");
  if (slash < 0) return null;
  const raw = rest.slice(slash);
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

/** 特殊键工具条：Ctrl 为待命修饰键，其余直接注入对应控制序列。 */
function sendKey(key: string) {
  if (key === "ctrl") {
    ctrlArmed.value = !ctrlArmed.value;
    term?.focus();
    return;
  }
  const seqs: Record<string, string> = {
    esc: "",
    tab: "\t",
    "ctrl-c": "",
    up: "[A",
    down: "[B",
    left: "[D",
    right: "[C",
  };
  const seq = seqs[key];
  if (seq) void sshWrite(props.opts.id, seq);
  term?.focus();
}

onMounted(async () => {
  const id = props.opts.id;
  term = new Terminal({
    fontFamily: "Consolas, 'Courier New', monospace",
    fontSize: 14,
    cursorBlink: true,
    theme: terminalTheme(props.theme),
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.loadAddon(new WebLinksAddon());
  term.open(host.value!);
  fit.fit();

  // 输出：后端推送 base64 编码的字节流。
  unlisten.push(
    await listen<string>(`terminal-output-${id}`, (e) => term?.write(b64ToBytes(e.payload)))
  );
  unlisten.push(await listen(`terminal-closed-${id}`, () => emit("closed")));

  // OSC 7：远端 shell 在每个提示符前汇报真实工作目录，去重后通知上层驱动 SFTP 跟随。
  const osc7 = term.parser.registerOscHandler(7, (data) => {
    const path = parseOsc7(data);
    if (path && path !== lastCwd) {
      lastCwd = path;
      emit("cwd", path);
    }
    return true; // 已处理，吞掉该序列不再交给终端渲染。
  });
  disposeOsc7 = () => osc7.dispose();

  // 输入与尺寸变化。Ctrl 待命时把下一个字母转为控制码（移动端特殊键条用）。
  term.onData((d) => {
    if (ctrlArmed.value && d.length === 1) {
      const code = d.toLowerCase().charCodeAt(0);
      if (code >= 97 && code <= 122) {
        ctrlArmed.value = false;
        return void sshWrite(id, String.fromCharCode(code - 96));
      }
      ctrlArmed.value = false;
    }
    void sshWrite(id, d);
  });
  term.onResize(({ cols, rows }) => void sshResize(id, cols, rows));

  // 用当前实测尺寸发起连接。
  try {
    await sshConnect({ ...props.opts, cols: term.cols, rows: term.rows });
    emit("connected");
    if (props.active !== false) term.focus();
  } catch (err) {
    emit("error", String(err));
  }
});

const ro = new ResizeObserver(() => fit?.fit());
onMounted(() => host.value && ro.observe(host.value));

watch(
  () => props.active,
  async (active) => {
    if (!active) return;
    await nextTick();
    requestAnimationFrame(() => {
      fit?.fit();
      term?.focus();
    });
  }
);

watch(
  () => props.theme,
  (theme) => {
    if (term) term.options.theme = terminalTheme(theme);
  }
);

onBeforeUnmount(() => {
  ro.disconnect();
  disposeOsc7?.();
  unlisten.forEach((fn) => fn());
  void sshDisconnect(props.opts.id);
  term?.dispose();
});
</script>

<template>
  <div class="term-wrap">
    <div ref="host" class="terminal"></div>
    <div v-if="isMobile" class="keybar">
      <button type="button" @click="sendKey('esc')">Esc</button>
      <button type="button" @click="sendKey('tab')">Tab</button>
      <button type="button" :class="{ armed: ctrlArmed }" @click="sendKey('ctrl')">Ctrl</button>
      <button type="button" @click="sendKey('ctrl-c')">^C</button>
      <button type="button" @click="sendKey('left')">←</button>
      <button type="button" @click="sendKey('up')">↑</button>
      <button type="button" @click="sendKey('down')">↓</button>
      <button type="button" @click="sendKey('right')">→</button>
    </div>
  </div>
</template>

<style scoped>
.term-wrap {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-height: 0;
}
.terminal {
  flex: 1;
  min-height: 0;
  width: 100%;
  background: var(--terminal-bg, #1e1e1e);
  padding: 4px;
  box-sizing: border-box;
}
.keybar {
  display: flex;
  gap: var(--sp-1);
  padding: var(--sp-1) var(--sp-2) calc(var(--sp-1) + var(--safe-bottom));
  background: var(--surface-2);
  border-top: 1px solid var(--line);
  overflow-x: auto;
}
.keybar button {
  flex: 1 0 auto;
  min-width: 44px;
  min-height: 40px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: var(--text);
  font-size: var(--fs-sm);
  cursor: pointer;
}
.keybar button.armed {
  border-color: var(--accent);
  background: var(--accent-soft);
}
</style>
