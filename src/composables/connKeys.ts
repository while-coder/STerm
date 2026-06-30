// 连接私钥在「加密载荷 ↔ 本地文件」之间的搬运辅助，导出与云同步共用。
// 加密载荷里携带私钥内容（privateKey），本地落盘后只保留路径（privateKeyPath）。
import {
  importPrivateKey,
  readTextFile,
  type ExportedConnection,
  type SavedConnection,
} from "../api";

/** 把每条 key 认证连接的私钥文件内容读入 privateKey 字段，供加密导出/上传。 */
export async function embedPrivateKeys(
  list: SavedConnection[]
): Promise<{ items: ExportedConnection[]; warnings: string[] }> {
  const warnings: string[] = [];
  const items: ExportedConnection[] = [];
  for (const c of list) {
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
  return { items, warnings };
}

/** 还原一条携带私钥的连接：写回私钥文件、改写 privateKeyPath，并剥离 privateKey。 */
export async function restorePrivateKey(
  item: ExportedConnection
): Promise<{ conn: SavedConnection; warning?: string }> {
  const { privateKey, ...rest } = item;
  const conn: SavedConnection = { ...rest };
  if (privateKey && conn.auth === "key") {
    try {
      conn.privateKeyPath = await importPrivateKey(conn.id, privateKey);
    } catch (e) {
      return {
        conn,
        warning: `「${conn.label}」私钥写入失败，请手动指定私钥路径：${String(e)}`,
      };
    }
  }
  return { conn };
}
