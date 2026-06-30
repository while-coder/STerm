// 机器列表导入 / 导出：导出时嵌入私钥内容并口令加密，导入时解密、回写私钥、合并到列表。
import { readTextFile, writeTextFile, type ExportedConnection } from "../api";
import { decryptJson, encryptJson } from "../crypto";
import { embedPrivateKeys, restorePrivateKey } from "./connKeys";
import { useConnections } from "./useConnections";
import { useSecurity } from "./useSecurity";

interface ExportPayload {
  connections: ExportedConnection[];
}

export interface PortabResult {
  count: number;
  warnings: string[];
}

export function usePortab() {
  const { connections, save } = useConnections();
  const { requireMasterPassword } = useSecurity();

  /** 导出全部连接到指定文件：嵌入私钥内容、口令加密。 */
  async function exportConnections(savePath: string): Promise<PortabResult> {
    const password = requireMasterPassword();
    const { items, warnings } = await embedPrivateKeys(connections.value);
    const payload: ExportPayload = { connections: items };
    const text = await encryptJson(payload, password, "connections-export");
    await writeTextFile(savePath, text);
    return { count: items.length, warnings };
  }

  /** 从文件导入连接：解密、回写私钥、按 id 合并到列表。 */
  async function importConnections(filePath: string): Promise<PortabResult> {
    const password = requireMasterPassword();
    const warnings: string[] = [];
    const text = await readTextFile(filePath);
    const payload = await decryptJson<ExportPayload>(text, password, "connections-export");

    const list = Array.isArray(payload?.connections) ? payload.connections : [];
    if (!list.length) throw new Error("文件中没有可导入的连接");

    let count = 0;
    for (const item of list) {
      const { conn, warning } = await restorePrivateKey(item);
      if (warning) warnings.push(warning);
      await save(conn);
      count++;
    }
    return { count, warnings };
  }

  return { exportConnections, importConnections };
}
