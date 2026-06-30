import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";
import { setupLogger, getLogger } from "./logger";

// 尽早初始化日志服务，使后续模块可直接 getLogger 使用。
setupLogger();

const log = getLogger("main");
log.info("应用启动");

// 兜底：统一记录未捕获异常与未处理的 Promise 拒绝，便于排查线上问题。
window.addEventListener("error", (e) => {
  log.error("未捕获异常", e.error ?? e.message);
});
window.addEventListener("unhandledrejection", (e) => {
  log.error("未处理的 Promise 拒绝", e.reason);
});

createApp(App).mount("#app");
