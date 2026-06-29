// 终端配色预设与字体选项。配色为 xterm 的 ITheme，"auto" 跟随界面深 / 浅主题。
import type { ITheme } from "@xterm/xterm";

export interface SchemeOption {
  key: string;
  label: string;
}

export const TERMINAL_SCHEMES: SchemeOption[] = [
  { key: "auto", label: "跟随界面主题" },
  { key: "vscode-dark", label: "VS Code 深色" },
  { key: "vscode-light", label: "VS Code 浅色" },
  { key: "solarized-dark", label: "Solarized Dark" },
  { key: "dracula", label: "Dracula" },
  { key: "one-light", label: "One Light" },
];

// 常用等宽字体（value 直接作为 CSS font-family）。
export const TERMINAL_FONTS: SchemeOption[] = [
  { key: "Consolas, 'Courier New', monospace", label: "Consolas" },
  { key: "'Cascadia Code', Consolas, monospace", label: "Cascadia Code" },
  { key: "'JetBrains Mono', Consolas, monospace", label: "JetBrains Mono" },
  { key: "Menlo, Monaco, monospace", label: "Menlo / Monaco" },
  { key: "'Courier New', monospace", label: "Courier New" },
];

const THEMES: Record<string, ITheme> = {
  "vscode-dark": { background: "#1e1e1e", foreground: "#d4d4d4", cursor: "#d4d4d4" },
  "vscode-light": { background: "#ffffff", foreground: "#1f2328", cursor: "#0b5cad" },
  "solarized-dark": {
    background: "#002b36",
    foreground: "#839496",
    cursor: "#93a1a1",
    selectionBackground: "#073642",
    black: "#073642",
    red: "#dc322f",
    green: "#859900",
    yellow: "#b58900",
    blue: "#268bd2",
    magenta: "#d33682",
    cyan: "#2aa198",
    white: "#eee8d5",
  },
  dracula: {
    background: "#282a36",
    foreground: "#f8f8f2",
    cursor: "#f8f8f2",
    selectionBackground: "#44475a",
    black: "#21222c",
    red: "#ff5555",
    green: "#50fa7b",
    yellow: "#f1fa8c",
    blue: "#bd93f9",
    magenta: "#ff79c6",
    cyan: "#8be9fd",
    white: "#f8f8f2",
  },
  "one-light": {
    background: "#fafafa",
    foreground: "#383a42",
    cursor: "#526fff",
    selectionBackground: "#e5e5e6",
    black: "#383a42",
    red: "#e45649",
    green: "#50a14f",
    yellow: "#c18401",
    blue: "#4078f2",
    magenta: "#a626a4",
    cyan: "#0184bc",
    white: "#fafafa",
  },
};

/** 解析配色方案为 xterm ITheme；auto 按界面主题回退到 VS Code 深 / 浅。 */
export function resolveTerminalTheme(scheme: string, appTheme: "dark" | "light"): ITheme {
  if (scheme === "auto") {
    return appTheme === "light" ? THEMES["vscode-light"] : THEMES["vscode-dark"];
  }
  return THEMES[scheme] ?? THEMES["vscode-dark"];
}

/** 配色方案对应的背景色（用于终端容器 padding 区域底色）。 */
export function schemeBackground(scheme: string, appTheme: "dark" | "light"): string {
  return resolveTerminalTheme(scheme, appTheme).background ?? "#1e1e1e";
}
