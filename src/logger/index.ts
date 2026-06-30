/**
 * 日志模块
 * 接口层（ILogger / ILoggerService / GlobalLoggerService）参考 sbot 设计，
 * 底层实现适配 Tauri：经 @tauri-apps/plugin-log 转发到 Rust 端，
 * 统一输出到标准输出与应用日志目录文件。
 */
import type { ILogger } from "./ILogger";
import { GlobalLoggerService } from "./GlobalLoggerService";
import { TauriLoggerService } from "./TauriLoggerService";

// ===== 接口 + 全局单例 =====
export type { ILogger } from "./ILogger";
export { ILoggerService } from "./ILoggerService";
export { GlobalLoggerService } from "./GlobalLoggerService";

let initialized = false;

/**
 * 初始化全局日志服务，应在应用入口尽早调用一次。
 */
export function setupLogger(): void {
  if (initialized) return;
  GlobalLoggerService.setLoggerService(new TauriLoggerService());
  initialized = true;
}

// 兜底：若调用方在 setupLogger 之前取 logger，则用控制台占位，避免抛错。
const consoleFallback = (name: string): ILogger => ({
  debug: (m, ...a) => console.debug(`[${name}]`, m, ...a),
  info: (m, ...a) => console.info(`[${name}]`, m, ...a),
  warn: (m, ...a) => console.warn(`[${name}]`, m, ...a),
  error: (m, ...a) => console.error(`[${name}]`, m, ...a),
});

/**
 * 获取指定名称的 logger（业务侧统一入口）。
 * @param name 模块名 / 文件名，作为日志来源前缀。
 */
export function getLogger(name: string): ILogger {
  return GlobalLoggerService.getLogger(name) ?? consoleFallback(name);
}
