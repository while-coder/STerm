<script setup lang="ts">
// xterm.js 终端：挂载后建立连接，桥接后端 PTY 数据流与键盘输入。
import { onMounted, onBeforeUnmount, ref } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import { sshConnect, sshWrite, sshResize, sshDisconnect, type ConnectOpts } from "../api";

const props = defineProps<{ opts: Omit<ConnectOpts, "cols" | "rows"> }>();
const emit = defineEmits<{ connected: []; error: [msg: string]; closed: [] }>();

const host = ref<HTMLDivElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
const unlisten: UnlistenFn[] = [];

/** base64 -> 字节数组，保证二进制流（含多字节 UTF-8）完整还原。 */
function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
  return arr;
}

onMounted(async () => {
  const id = props.opts.id;
  term = new Terminal({
    fontFamily: "Consolas, 'Courier New', monospace",
    fontSize: 14,
    cursorBlink: true,
    theme: { background: "#1e1e1e", foreground: "#d4d4d4" },
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
  term.onData((d) => void sshWrite(id, d));
  term.onResize(({ cols, rows }) => void sshResize(id, cols, rows));

  // 用当前实测尺寸发起连接。
  try {
    await sshConnect({ ...props.opts, cols: term.cols, rows: term.rows });
    emit("connected");
    term.focus();
  } catch (err) {
    emit("error", String(err));
  }
});

const ro = new ResizeObserver(() => fit?.fit());
onMounted(() => host.value && ro.observe(host.value));

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
  background: #1e1e1e;
  padding: 4px;
  box-sizing: border-box;
}
</style>
