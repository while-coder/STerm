// 机器列表的 GitHub Gist 同步：pull → 合并 → 落盘 + 回写私钥 → push。
// 加解密与合并都在前端；后端只搬运密文并从系统凭证库读取 PAT。模块级单例。
import { ref, watch } from "vue";
import {
  deleteCredential,
  gistFind,
  gistPull,
  gistPush,
  gistValidate,
  setCredential,
  type ExportedConnection,
  type SavedConnection,
} from "../api";
import { decryptJson, encryptJson } from "../crypto";
import type { ConnectionsState, Tombstones } from "../storage";
import { embedPrivateKeys, restorePrivateKey } from "./connKeys";
import { mergeConnections } from "./syncMerge";
import { useConnections } from "./useConnections";
import { useSecurity } from "./useSecurity";
import { useSettings } from "./useSettings";

/** 系统凭证库中存放 PAT 的 key。 */
const PAT_KEY = "github-pat";
/** 改动后推送的防抖时长（毫秒）。 */
const PUSH_DEBOUNCE_MS = 3000;

interface SyncPayload {
  connections: ExportedConnection[];
  tombstones: Tombstones;
}

const { settings } = useSettings();
const { connections, tombstones, changeTick, applyMerged } = useConnections();
const { requireMasterPassword } = useSecurity();

const syncing = ref(false);
const lastError = ref("");

function toMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** 执行一次完整同步（pull → 合并 → 落盘 → push）。 */
async function syncNow(): Promise<void> {
  if (syncing.value || !settings.syncEnabled) return;
  const password = requireMasterPassword();
  syncing.value = true;
  lastError.value = "";
  try {
    // 1. 拉取远端（含私钥内容的载荷）。
    let remoteState: ConnectionsState = { connections: [], tombstones: {} };
    if (settings.syncGistId) {
      const pull = await gistPull(settings.syncGistId);
      if (pull.content) {
        const payload = await decryptJson<SyncPayload>(
          pull.content,
          password,
          "connections-sync"
        );
        remoteState = {
          connections: Array.isArray(payload?.connections) ? payload.connections : [],
          tombstones:
            payload?.tombstones && typeof payload.tombstones === "object"
              ? payload.tombstones
              : {},
        };
      }
    }

    // 2. 本地状态（不含私钥内容，仅路径）与远端按 id 逐条合并。
    //    首次接入同步（从未成功同步过）时忽略本地删除墓碑：接入前的本地删除不应
    //    压制远端已有数据，否则会误删另一台设备的连接。此时合并取并集，之后才增量删除。
    const firstSync = settings.syncLastAt === 0;
    const localState: ConnectionsState = {
      connections: connections.value,
      tombstones: firstSync ? {} : tombstones.value,
    };
    const merged = mergeConnections(localState, remoteState);

    // 3. 对来自远端、携带私钥内容的条目回写私钥文件并修正路径；本地条目保持原样。
    const cleanConns: SavedConnection[] = [];
    for (const c of merged.connections) {
      if ((c as ExportedConnection).privateKey) {
        const { conn } = await restorePrivateKey(c as ExportedConnection);
        cleanConns.push(conn);
      } else {
        cleanConns.push(c);
      }
    }
    await applyMerged({ connections: cleanConns, tombstones: merged.tombstones });

    // 4. 把合并结果（重新从磁盘读入私钥内容）加密后推送，gistId 为空则新建。
    const { items } = await embedPrivateKeys(cleanConns);
    const text = await encryptJson(
      { connections: items, tombstones: merged.tombstones } satisfies SyncPayload,
      password,
      "connections-sync"
    );
    const push = await gistPush(settings.syncGistId || null, text);
    settings.syncGistId = push.gistId;
    settings.syncLastAt = Date.now();
  } catch (e) {
    lastError.value = toMessage(e);
    throw e;
  } finally {
    syncing.value = false;
  }
}

/** 配置并启用同步：验证 PAT → 存入凭证库 → 立即同步（首次会创建 gist）。 */
async function configure(pat: string, gistId?: string): Promise<string> {
  const token = pat.trim();
  if (!token) throw new Error("请填写 GitHub PAT");
  const login = await gistValidate(token);
  await setCredential(PAT_KEY, token);
  settings.syncEnabled = true;
  if (gistId && gistId.trim()) {
    settings.syncGistId = gistId.trim();
  } else if (!settings.syncGistId) {
    // 未手动指定时，自动复用同账号下已有的 STerm gist；找不到才在 syncNow 里新建。
    const found = await gistFind().catch(() => null);
    if (found) settings.syncGistId = found;
  }
  await syncNow();
  return login;
}

/** 断开同步：删除 PAT 与本地同步配置（不动机器列表本身）。 */
async function disconnect(): Promise<void> {
  await deleteCredential(PAT_KEY).catch(() => undefined);
  settings.syncEnabled = false;
  settings.syncGistId = "";
  settings.syncLastAt = 0;
  lastError.value = "";
}

// 改动后防抖推送：仅在已启用且已有 gist 时触发。
let timer: ReturnType<typeof setTimeout> | null = null;
watch(changeTick, () => {
  if (!settings.syncEnabled || !settings.syncGistId) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    timer = null;
    syncNow().catch(() => undefined);
  }, PUSH_DEBOUNCE_MS);
});

export function useGistSync() {
  return { syncing, lastError, syncNow, configure, disconnect };
}
