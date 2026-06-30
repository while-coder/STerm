import type { ILogger } from "./ILogger";
import type { ILoggerService } from "./ILoggerService";

/**
 * 全局 Logger 服务持有者。
 * 在应用启动时注入具体实现，业务侧统一从这里取 logger。
 */
class LoggerServiceHolder {
  private loggerService: ILoggerService | undefined;

  setLoggerService(service: ILoggerService): void {
    this.loggerService = service;
  }

  getLoggerService(): ILoggerService | undefined {
    return this.loggerService;
  }

  getLogger(name: string): ILogger | undefined {
    return this.loggerService?.getLogger(name);
  }
}

/**
 * 全局 Logger 服务单例。
 */
export const GlobalLoggerService = new LoggerServiceHolder();
