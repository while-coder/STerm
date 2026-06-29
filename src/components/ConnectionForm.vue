<script setup lang="ts">
// 连接配置表单：新建或编辑一条连接，支持保存 / 保存并连接。
import { onMounted, reactive, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { defaultPrivateKeyPath, type AuthMethod, type SavedConnection } from "../api";
import { useConnections } from "../composables/useConnections";

const props = defineProps<{
  connection?: SavedConnection | null;
  error?: string;
}>();
const emit = defineEmits<{
  save: [conn: SavedConnection];
  connect: [conn: SavedConnection];
  cancel: [];
}>();

const FALLBACK_KEY = "~/.ssh/id_rsa";
const { groupNames } = useConnections();
const defaultKeyPath = ref(FALLBACK_KEY);
const localError = ref("");

const form = reactive({
  id: "",
  host: "",
  port: 22,
  username: "",
  auth: "password" as AuthMethod,
  password: "",
  privateKeyPath: FALLBACK_KEY,
  passphrase: "",
  remember: false,
  group: "",
  favorite: false,
});

function fill(c: SavedConnection) {
  form.id = c.id;
  form.host = c.host;
  form.port = c.port;
  form.username = c.username;
  form.auth = c.auth;
  form.privateKeyPath = c.privateKeyPath ?? defaultKeyPath.value;
  form.remember = c.remember;
  form.password = c.remember ? c.password ?? "" : "";
  form.passphrase = c.remember ? c.passphrase ?? "" : "";
  form.group = c.group ?? "";
  form.favorite = c.favorite ?? false;
}

onMounted(async () => {
  try {
    const path = await defaultPrivateKeyPath();
    defaultKeyPath.value = path || FALLBACK_KEY;
  } catch {
    defaultKeyPath.value = FALLBACK_KEY;
  }
  if (props.connection) {
    fill(props.connection);
  } else {
    form.privateKeyPath = defaultKeyPath.value;
  }
});

async function pickKey() {
  const p = await open({ multiple: false });
  if (typeof p === "string") form.privateKeyPath = p;
}

function build(): SavedConnection | null {
  const host = form.host.trim();
  const username = form.username.trim();
  if (!host || !username) {
    localError.value = "保存前请填写主机和用户名";
    return null;
  }
  localError.value = "";
  return {
    id: form.id || crypto.randomUUID(),
    label: `${username}@${host}`,
    host,
    port: Number(form.port),
    username,
    auth: form.auth,
    privateKeyPath: form.auth === "key" ? form.privateKeyPath : undefined,
    remember: form.remember,
    password: form.auth === "password" ? form.password : undefined,
    passphrase: form.auth === "key" ? form.passphrase || undefined : undefined,
    group: form.group.trim() || undefined,
    favorite: form.favorite,
  };
}

function onSave() {
  const conn = build();
  if (conn) emit("save", conn);
}
function onConnect() {
  const conn = build();
  if (conn) emit("connect", conn);
}
</script>

<template>
  <form class="form" @submit.prevent="onConnect">
    <div class="row">
      <input v-model="form.host" placeholder="主机 (host)" required />
      <input v-model.number="form.port" type="number" placeholder="端口" class="port" />
    </div>
    <input v-model="form.username" placeholder="用户名" required />

    <div class="row">
      <input v-model="form.group" list="conn-groups" placeholder="分组（可选）" />
      <label class="fav-toggle" :class="{ on: form.favorite }">
        <input v-model="form.favorite" type="checkbox" />
        ★ 收藏
      </label>
    </div>
    <datalist id="conn-groups">
      <option v-for="g in groupNames" :key="g" :value="g" />
    </datalist>

    <div class="auth-tabs">
      <label><input v-model="form.auth" type="radio" value="password" /> 密码</label>
      <label><input v-model="form.auth" type="radio" value="key" /> 私钥</label>
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

    <label class="remember">
      <input v-model="form.remember" type="checkbox" />
      记住密码 / 口令
    </label>

    <p v-if="localError || props.error" class="error">{{ localError || props.error }}</p>

    <div class="actions">
      <button type="button" @click="emit('cancel')">取消</button>
      <button type="button" @click="onSave">保存到列表</button>
      <button type="submit" class="primary">保存并连接</button>
    </div>
  </form>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.row {
  display: flex;
  gap: var(--sp-2);
}
.row input:first-child {
  flex: 1;
}
.port {
  width: 90px;
}
.auth-tabs {
  display: flex;
  gap: var(--sp-5);
  padding: var(--sp-1) 0;
}
.auth-tabs label {
  display: inline-flex;
  align-items: center;
  gap: var(--sp-2);
  cursor: pointer;
}
.auth-tabs input {
  margin: 0;
}
.fav-toggle {
  display: flex;
  align-items: center;
  gap: var(--sp-1);
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--muted);
  font-size: var(--fs-sm);
  cursor: pointer;
  white-space: nowrap;
}
.fav-toggle.on {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--text);
}
.fav-toggle input {
  display: none;
}
.remember {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  font-size: var(--fs-sm);
  color: var(--muted);
}
.remember input,
.auth-tabs input {
  width: auto;
}
.actions {
  display: flex;
  gap: var(--sp-2);
  margin-top: var(--sp-1);
}
.actions .primary {
  flex: 1;
}
.error {
  margin: 0;
  color: var(--warn);
  font-size: var(--fs-sm);
}
input:not([type="radio"]):not([type="checkbox"]) {
  min-height: var(--hit);
  padding: 0 var(--sp-3);
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-md);
}
button {
  min-height: var(--hit);
  padding: 0 var(--sp-4);
  border-radius: var(--radius);
  border: 1px solid var(--line);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
button:hover {
  border-color: var(--accent);
}
.primary {
  background: var(--accent);
  border-color: var(--accent);
}
.primary:hover {
  background: var(--accent-hover);
}
</style>
