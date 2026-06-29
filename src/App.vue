<script setup lang="ts">
import { reactive, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import TerminalView from "./components/TerminalView.vue";
import FileBrowser from "./components/FileBrowser.vue";
import type { AuthMethod, ConnectOpts } from "./api";

type Phase = "form" | "session";
const phase = ref<Phase>("form");
const connected = ref(false);
const connectError = ref("");

// 当前会话的连接参数（不含终端尺寸，尺寸由终端挂载后实测）。
const session = ref<Omit<ConnectOpts, "cols" | "rows"> | null>(null);

const form = reactive({
  host: "",
  port: 22,
  username: "",
  auth: "password" as AuthMethod,
  password: "",
  privateKeyPath: "",
  passphrase: "",
});

async function pickKey() {
  const p = await open({ multiple: false });
  if (typeof p === "string") form.privateKeyPath = p;
}

function connect() {
  connectError.value = "";
  session.value = {
    id: crypto.randomUUID(),
    host: form.host.trim(),
    port: Number(form.port),
    username: form.username.trim(),
    auth: form.auth,
    password: form.auth === "password" ? form.password : undefined,
    privateKeyPath: form.auth === "key" ? form.privateKeyPath : undefined,
    passphrase: form.auth === "key" ? form.passphrase || undefined : undefined,
  };
  connected.value = false;
  phase.value = "session";
}

function disconnect() {
  phase.value = "form";
  connected.value = false;
  session.value = null;
}

function onError(msg: string) {
  connectError.value = msg;
  phase.value = "form";
  session.value = null;
}
</script>

<template>
  <!-- 连接表单 -->
  <div v-if="phase === 'form'" class="form-wrap">
    <h1>STerm</h1>
    <p class="sub">SSH / SFTP 终端</p>
    <form class="form" @submit.prevent="connect">
      <div class="row">
        <input v-model="form.host" placeholder="主机 (host)" required />
        <input v-model.number="form.port" type="number" placeholder="端口" class="port" />
      </div>
      <input v-model="form.username" placeholder="用户名" required />

      <div class="auth-tabs">
        <label><input type="radio" value="password" v-model="form.auth" /> 密码</label>
        <label><input type="radio" value="key" v-model="form.auth" /> 私钥</label>
      </div>

      <input
        v-if="form.auth === 'password'"
        v-model="form.password"
        type="password"
        placeholder="密码"
      />
      <template v-else>
        <div class="row">
          <input v-model="form.privateKeyPath" placeholder="私钥文件路径" />
          <button type="button" @click="pickKey">选择…</button>
        </div>
        <input v-model="form.passphrase" type="password" placeholder="私钥口令（可选）" />
      </template>

      <button type="submit" class="primary">连接</button>
      <p v-if="connectError" class="error">{{ connectError }}</p>
    </form>
  </div>

  <!-- 会话视图：终端 + 文件浏览器 -->
  <div v-else class="session">
    <div class="bar">
      <span>{{ session?.username }}@{{ session?.host }}</span>
      <button @click="disconnect">断开</button>
    </div>
    <div class="split">
      <div class="pane term-pane">
        <TerminalView
          v-if="session"
          :opts="session"
          @connected="connected = true"
          @error="onError"
          @closed="disconnect"
        />
      </div>
      <div class="pane file-pane">
        <FileBrowser v-if="session" :id="session.id" :active="connected" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.form-wrap {
  max-width: 380px;
  margin: 8vh auto;
  text-align: center;
}
h1 {
  margin: 0;
  font-size: 2.2rem;
}
.sub {
  opacity: 0.6;
  margin: 4px 0 24px;
}
.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  text-align: left;
}
.row {
  display: flex;
  gap: 8px;
}
.row input:first-child {
  flex: 1;
}
.port {
  width: 90px;
}
.auth-tabs {
  display: flex;
  gap: 18px;
  padding: 4px 0;
}
input {
  padding: 0.6em 0.8em;
  border-radius: 8px;
  border: 1px solid #444;
  background: #2a2a2a;
  color: #eee;
  font-size: 1em;
}
button {
  padding: 0.55em 1em;
  border-radius: 8px;
  border: 1px solid #555;
  background: #333;
  color: #eee;
  cursor: pointer;
}
button:hover {
  border-color: #569cd6;
}
.primary {
  background: #0e639c;
  border-color: #0e639c;
}
.primary:hover {
  background: #1177bb;
}
.error {
  color: #f48771;
}

.session {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  background: #323233;
  color: #ddd;
  font-size: 13px;
}
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}
.pane {
  min-width: 0;
}
.term-pane {
  flex: 1;
}
.file-pane {
  width: 340px;
  border-left: 1px solid #333;
}
</style>
