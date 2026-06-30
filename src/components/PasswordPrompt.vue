<script setup lang="ts">
// 口令弹窗：导出模式要求设置口令并二次确认；导入模式只需输入口令。
import { computed, ref } from "vue";
import BaseSheet from "./BaseSheet.vue";

const props = defineProps<{ mode: "export" | "import" }>();
const emit = defineEmits<{ confirm: [password: string]; cancel: [] }>();

const password = ref("");
const confirm = ref("");
const localError = ref("");

const isExport = computed(() => props.mode === "export");
const title = computed(() => (isExport.value ? "导出机器列表" : "导入机器列表"));
const subtitle = computed(() =>
  isExport.value ? "设置一个口令用于加密导出文件，导入时需要它" : "输入导出时设置的口令以解密文件"
);

function onConfirm() {
  const pwd = password.value;
  if (!pwd) {
    localError.value = "请输入口令";
    return;
  }
  if (isExport.value && pwd !== confirm.value) {
    localError.value = "两次输入的口令不一致";
    return;
  }
  localError.value = "";
  emit("confirm", pwd);
}
</script>

<template>
  <BaseSheet :title="title" :subtitle="subtitle" @close="emit('cancel')">
    <form class="form" @submit.prevent="onConfirm">
      <input
        v-model="password"
        type="password"
        :placeholder="isExport ? '设置口令' : '口令'"
      />
      <input
        v-if="isExport"
        v-model="confirm"
        type="password"
        placeholder="再次输入口令"
      />

      <p v-if="isExport" class="hint">⚠ 导出文件包含密码与私钥，口令丢失后将无法解密，请妥善保管。</p>
      <p v-if="localError" class="error">{{ localError }}</p>

      <div class="actions">
        <button type="button" @click="emit('cancel')">取消</button>
        <button type="submit" class="primary">{{ isExport ? "导出" : "导入" }}</button>
      </div>
    </form>
  </BaseSheet>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
.hint {
  margin: 0;
  color: var(--muted);
  font-size: var(--fs-sm);
}
.error {
  margin: 0;
  color: var(--warn);
  font-size: var(--fs-sm);
}
.actions {
  display: flex;
  gap: var(--sp-2);
  margin-top: var(--sp-1);
}
.actions .primary {
  flex: 1;
}
input {
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
  color: #fff;
}
.primary:hover {
  background: var(--accent-hover);
}
</style>
