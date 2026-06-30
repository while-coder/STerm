// 机器列表多设备同步的合并算法：纯函数，按 id 逐条取最新。
// 每条连接带 updatedAt，删除记为墓碑（tombstone）。某个 id 的「修改」与「删除」
// 谁的时间戳更晚谁胜出；删除信息以墓碑形式继续传播，超过 TTL 后回收。
import type { SavedConnection } from "../api";
import type { ConnectionsState, Tombstones } from "../storage";

/** 墓碑保留时长（90 天）。超过后回收，避免无限增长。 */
export const TOMBSTONE_TTL_MS = 90 * 24 * 60 * 60 * 1000;

function connTime(c: SavedConnection | undefined): number {
  return c?.updatedAt ?? 0;
}

/** 取两条同 id 连接中较新的一条；updatedAt 相等时偏向本地。 */
function pickNewer(
  local: SavedConnection | undefined,
  remote: SavedConnection | undefined
): SavedConnection {
  if (!local) return remote as SavedConnection;
  if (!remote) return local;
  return connTime(remote) > connTime(local) ? remote : local;
}

/**
 * 合并本地与远端的机器列表状态。
 * 对每个出现过的 id，比较「最新修改时间」与「最新删除时间」：
 * - 删除时间严格更晚 → 删除（保留墓碑）。
 * - 否则 → 保留较新的连接。
 * 合并完成后回收过期墓碑。
 */
export function mergeConnections(
  local: ConnectionsState,
  remote: ConnectionsState,
  now: number = Date.now()
): ConnectionsState {
  const localConns = new Map(local.connections.map((c) => [c.id, c]));
  const remoteConns = new Map(remote.connections.map((c) => [c.id, c]));

  const ids = new Set<string>([
    ...localConns.keys(),
    ...remoteConns.keys(),
    ...Object.keys(local.tombstones),
    ...Object.keys(remote.tombstones),
  ]);

  const connections: SavedConnection[] = [];
  const tombstones: Tombstones = {};

  for (const id of ids) {
    const lc = localConns.get(id);
    const rc = remoteConns.get(id);
    const lt = local.tombstones[id];
    const rt = remote.tombstones[id];

    const hasConn = lc !== undefined || rc !== undefined;
    const hasTomb = lt !== undefined || rt !== undefined;
    const connTs = Math.max(connTime(lc), connTime(rc));
    const tombTs = Math.max(lt ?? 0, rt ?? 0);

    if (hasTomb && (!hasConn || tombTs > connTs)) {
      // 删除胜出：仅在未过期时保留墓碑继续传播。
      if (now - tombTs <= TOMBSTONE_TTL_MS) tombstones[id] = tombTs;
    } else if (hasConn) {
      connections.push(pickNewer(lc, rc));
    }
  }

  return { connections, tombstones };
}
