// 传输管理器：统一调度上传 / 下载，带并发上限与（下载）字节级进度。模块级单例。
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { sftpCancel } from "../api";
import { useSettings } from "./useSettings";

export type TransferKind = "download" | "upload";
export type TransferStatus = "queued" | "running" | "done" | "error" | "cancelled";

export interface Transfer {
  id: string;
  name: string;
  kind: TransferKind;
  status: TransferStatus;
  transferred: number;
  total: number;
  error?: string;
  /** 所属会话（用于按 session 过滤 / 分组）。 */
  sessionId: string;
  sessionLabel: string;
}

interface ProgressPayload {
  transferred: number;
  total: number;
  done: boolean;
}

interface PendingTask {
  transfer: Transfer;
  withProgress: boolean;
  start: (transferId: string) => Promise<void>;
}

const { settings } = useSettings();
const transfers = ref<Transfer[]>([]);
const queue: PendingTask[] = [];
const cancelledIds = new Set<string>();
let running = 0;

function pump() {
  const max = Math.max(1, settings.maxParallelTransfers || 1);
  while (running < max && queue.length) {
    const task = queue.shift()!;
    void runTask(task);
  }
}

async function runTask(task: PendingTask) {
  const { transfer, withProgress, start } = task;
  running += 1;
  transfer.status = "running";
  let unlisten: (() => void) | null = null;
  if (withProgress) {
    unlisten = await listen<ProgressPayload>(`sftp-progress-${transfer.id}`, (e) => {
      transfer.transferred = e.payload.transferred;
      transfer.total = e.payload.total;
    });
  }
  try {
    await start(transfer.id);
    transfer.status = "done";
  } catch (err) {
    if (cancelledIds.has(transfer.id)) {
      transfer.status = "cancelled";
      cancelledIds.delete(transfer.id);
    } else {
      transfer.status = "error";
      transfer.error = String(err);
    }
  } finally {
    unlisten?.();
    running -= 1;
    pump();
  }
}

/** 取消传输：排队中的直接移除；进行中的发取消令牌（后端中止并清理半成品）。 */
async function cancel(id: string) {
  const transfer = transfers.value.find((t) => t.id === id);
  if (!transfer) return;
  if (transfer.status === "queued") {
    const i = queue.findIndex((q) => q.transfer.id === id);
    if (i >= 0) queue.splice(i, 1);
    transfer.status = "cancelled";
    return;
  }
  if (transfer.status === "running") {
    cancelledIds.add(id);
    try {
      await sftpCancel(id);
    } catch {
      /* 忽略 */
    }
  }
}

/** 入队一个传输任务。start 收到 transferId，需调用对应后端命令。 */
function enqueue(opts: {
  kind: TransferKind;
  name: string;
  sessionId: string;
  sessionLabel: string;
  withProgress?: boolean;
  start: (transferId: string) => Promise<void>;
}): string {
  const transfer: Transfer = {
    id: crypto.randomUUID(),
    name: opts.name,
    kind: opts.kind,
    status: "queued",
    transferred: 0,
    total: 0,
    sessionId: opts.sessionId,
    sessionLabel: opts.sessionLabel,
  };
  transfers.value.push(transfer);
  queue.push({ transfer, withProgress: opts.withProgress ?? false, start: opts.start });
  pump();
  return transfer.id;
}

/** 清除已完成 / 失败的记录。 */
function clearFinished() {
  transfers.value = transfers.value.filter(
    (t) => t.status === "queued" || t.status === "running"
  );
}

export function useTransfers() {
  return { transfers, enqueue, cancel, clearFinished };
}
