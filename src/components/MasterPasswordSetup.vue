<script setup lang="ts">
// 首次启动时设置主密码，并保存到系统凭证。
import { ref } from "vue";
import { useSecurity } from "../composables/useSecurity";

defineProps<{ initialError?: string }>();

const { createMasterPassword } = useSecurity();
const password = ref("");
const confirm = ref("");
const busy = ref(false);
const localError = ref("");

async function onSubmit() {
  if (!password.value) {
    localError.value = "请输入主密码";
    return;
  }
  if (password.value !== confirm.value) {
    localError.value = "两次输入的主密码不一致";
    return;
  }
  busy.value = true;
  localError.value = "";
  try {
    await createMasterPassword(password.value);
  } catch (e) {
    localError.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <main class="setup">
    <form class="panel" @submit.prevent="onSubmit">
      <header>
        <h1>设置主密码</h1>
        <p>STerm 会把主密码保存到系统凭证，并用它加密本地机器列表。</p>
      </header>

      <input v-model="password" type="password" placeholder="主密码" autocomplete="new-password" />
      <input
        v-model="confirm"
        type="password"
        placeholder="再次输入主密码"
        autocomplete="new-password"
      />

      <p v-if="initialError" class="hint">{{ initialError }}</p>
      <p v-if="localError" class="error">{{ localError }}</p>

      <button type="submit" class="primary" :disabled="busy">
        {{ busy ? "保存中…" : "开始使用" }}
      </button>
    </form>
  </main>
</template>

<style scoped>
.setup {
  display: grid;
  place-items: center;
  min-height: 100vh;
  padding: var(--sp-5);
  background: var(--bg);
}
.panel {
  width: min(420px, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
  padding: var(--sp-5);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface);
  box-shadow: var(--shadow);
}
header {
  margin-bottom: var(--sp-1);
}
h1 {
  margin: 0;
  font-size: var(--fs-xl);
}
p {
  margin: var(--sp-2) 0 0;
  color: var(--muted);
  font-size: var(--fs-sm);
}
.hint {
  margin: 0;
  color: var(--muted);
}
.error {
  margin: 0;
  color: var(--warn);
}
input {
  min-height: var(--hit);
  padding: 0 var(--sp-3);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-2);
  color: var(--text);
  font-size: var(--fs-md);
}
button {
  min-height: var(--hit);
  padding: 0 var(--sp-4);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  background: var(--surface-3);
  color: var(--text);
  cursor: pointer;
}
button:disabled {
  opacity: 0.7;
  cursor: default;
}
.primary {
  border-color: var(--accent);
  background: var(--accent);
  color: #fff;
}
.primary:not(:disabled):hover {
  background: var(--accent-hover);
}
</style>
