// 机器列表导入 / 导出：导出时嵌入私钥内容并口令加密，导入时解密、回写私钥、合并到列表。
import {
  importPrivateKey,
  readTextFile,
  writeTextFile,
  type ExportedConnection,
  type SavedConnection,
} from "../api";
import { decryptJson, encryptJson } from "../crypto";
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
    const warnings: string[] = [];
    const items: ExportedConnection[] = [];

    for (const c of connections.value) {
      const entry: ExportedConnection = { ...c };
      if (c.auth === "key" && c.privateKeyPath) {
        try {
          entry.privateKey = await readTextFile(c.privateKeyPath);
        } catch (e) {
          warnings.push(`「${c.label}」私钥读取失败，已跳过私钥内容：${String(e)}`);
        }
      }
      items.push(entry);
    }

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
      const { privateKey, ...rest } = item;
      const conn: SavedConnection = { ...rest };
      if (privateKey && conn.auth === "key") {
        try {
          conn.privateKeyPath = await importPrivateKey(conn.id, privateKey);
        } catch (e) {
          warnings.push(`「${conn.label}」私钥写入失败，请手动指定私钥路径：${String(e)}`);
        }
      }
      await save(conn);
      count++;
    }
    return { count, warnings };
  }

  return { exportConnections, importConnections };
}
