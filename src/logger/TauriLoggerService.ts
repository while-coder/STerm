import { debug, info, warn, error } from "@tauri-apps/plugin-log";
import type { ILogger } from "./ILogger";
import type { ILoggerService } from "./ILoggerService";

/**
 * 把附加参数拼接到消息末尾，便于在纯文本日志文件中查看。
 * 字符串原样输出；Error 取栈信息；其余用 JSON 序列化，失败则退化为 String()。
 */
function formatArgs(args: unknown[]): string {
  if (args.length === 0) return "";
  const parts = args.map((a) => {
    if (typeof a === "string") return a;
    if (a instanceof Error) return a.stack ?? `${a.name}: ${a.message}`;
    try {
      return JSON.stringify(a);
    } catch {
      return String(a);
    }
  });
  return " " + parts.join(" ");
}

/**
 * 基于 @tauri-apps/plugin-log 的 Logger 适配器。
 * 日志经插件转发到 Rust 端，统一输出到标准输出与日志文件。
 * name 作为来源前缀写入消息（对应 sbot 中的 category）。
 */
class TauriLoggerAdapter implements ILogger {
  constructor(private readonly name: string) {}

  private format(message: string, args: unknown[]): string {
    return `[${this.name}] ${message}${formatArgs(args)}`;
  }

  debug(message: string, ...args: unknown[]): void {
    void debug(this.format(message, args));
  }

  info(message: string, ...args: unknown[]): void {
    void info(this.format(message, args));
  }

  warn(message: string, ...args: unknown[]): void {
    void warn(this.format(message, args));
  }

  error(message: string, ...args: unknown[]): void {
    void error(this.format(message, args));
  }
}

/**
 * 基于 Tauri 的 LoggerService 实现。
 */
export class TauriLoggerService implements ILoggerService {
  getLogger(name: string): ILogger {
    return new TauriLoggerAdapter(name);
  }
}
