<script setup lang="ts">
// xterm.js 终端：挂载后建立连接，桥接后端 PTY 数据流与键盘输入。
import { nextTick, onMounted, onBeforeUnmount, ref, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { sshConnect, sshWrite, sshResize, sshDisconnect, type ConnectOpts } from "../api";

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
  cwdCommand: [path: string];
}>();

const host = ref<HTMLDivElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
const unlisten: UnlistenFn[] = [];
let inputBuffer = "";
let ignoreNextLf = false;

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

function shellWords(command: string): string[] {
  const words: string[] = [];
  let current = "";
  let quote: "'" | '"' | null = null;
  let escaped = false;
  for (const ch of command) {
    if (escaped) {
      current += ch;
      escaped = false;
      continue;
    }
    if (ch === "\\") {
      escaped = true;
      continue;
    }
    if (quote) {
      if (ch === quote) {
        quote = null;
      } else {
        current += ch;
      }
      continue;
    }
    if (ch === "'" || ch === '"') {
      quote = ch;
      continue;
    }
    if (/\s/.test(ch)) {
      if (current) {
        words.push(current);
        current = "";
      }
      continue;
    }
    current += ch;
  }
  if (current) words.push(current);
  return words;
}

function extractCdTarget(command: string): string | null {
  const trimmed = command.trim();
  if (!trimmed || /[;&|`$()]/.test(trimmed)) return null;
  const words = shellWords(trimmed);
  if (words[0] !== "cd") return null;
  const target = words.find((word, index) => index > 0 && word !== "--");
  if (!target || target === "-") return target === undefined ? "~" : null;
  return target;
}

function submitInputBuffer() {
  const target = extractCdTarget(inputBuffer);
  inputBuffer = "";
  if (target) emit("cwdCommand", target);
}

function trackInput(data: string) {
  for (const ch of data) {
    if (ch === "\r") {
      submitInputBuffer();
      ignoreNextLf = true;
      continue;
    }
    if (ch === "\n") {
      if (ignoreNextLf) {
        ignoreNextLf = false;
      } else {
        submitInputBuffer();
      }
      continue;
    }
    ignoreNextLf = false;
    if (ch === "\u007f" || ch === "\b") {
      inputBuffer = inputBuffer.slice(0, -1);
      continue;
    }
    if (ch === "\u0003" || ch === "\u0015") {
      inputBuffer = "";
      continue;
    }
    if (ch >= " ") inputBuffer += ch;
  }
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

  // 输入与尺寸变化。
  term.onData((d) => {
    trackInput(d);
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
  unlisten.forEach((fn) => fn());
  void sshDisconnect(props.opts.id);
  term?.dispose();
});
</script>

<template>
  <div ref="host" class="terminal"></div>
</template>

<style scoped>
.terminal {
  width: 100%;
  height: 100%;
  background: var(--terminal-bg, #1e1e1e);
  padding: 4px;
  box-sizing: border-box;
}
</style>
