// 应用设置：响应式状态 + 持久化 + 主题解析（含跟随系统）。模块级单例。
import { reactive, ref, watch } from "vue";
import { loadSettings, saveSettings, type AppSettings } from "../storage";

export type ResolvedTheme = "dark" | "light";

const settings = reactive<AppSettings>(loadSettings());
const resolvedTheme = ref<ResolvedTheme>("dark");

let themeQuery: MediaQueryList | null = null;
let initialized = false;

function resolveTheme(): ResolvedTheme {
  if (settings.theme === "dark" || settings.theme === "light") return settings.theme;
  return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function applyTheme() {
  resolvedTheme.value = resolveTheme();
  document.documentElement.dataset.theme = resolvedTheme.value;
}

/** 在应用根组件挂载时调用一次，建立主题监听与持久化。 */
function initSettings() {
  if (initialized) return;
  initialized = true;
  applyTheme();
  themeQuery = window.matchMedia("(prefers-color-scheme: light)");
  themeQuery.addEventListener("change", applyTheme);
  watch(() => settings.theme, applyTheme);
  watch(settings, () => saveSettings(settings), { deep: true });
}

export function useSettings() {
  return { settings, resolvedTheme, applyTheme, initSettings };
}
