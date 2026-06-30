// 机器列表本地存储与导入 / 导出文件的口令加密：PBKDF2 派生密钥 + AES-GCM-256。
// 加密结果是一个自描述的 JSON 信封，包含 KDF 参数、盐、IV 与密文（均 base64）。

const PBKDF2_ITERATIONS = 200_000;
const SALT_BYTES = 16;
const IV_BYTES = 12;

export type EncryptedEnvelopeType =
  | "connections-store"
  | "connections-export"
  | "connections-sync";

export interface EncryptedEnvelope {
  app: "STerm";
  type: EncryptedEnvelopeType;
  version: 1;
  kdf: { algo: "PBKDF2"; hash: "SHA-256"; iterations: number; salt: string };
  cipher: "AES-GCM";
  iv: string;
  data: string;
}

function toBase64(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

function fromBase64(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

async function deriveKey(
  password: string,
  salt: Uint8Array,
  iterations: number
): Promise<CryptoKey> {
  const baseKey = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(password),
    "PBKDF2",
    false,
    ["deriveKey"]
  );
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", hash: "SHA-256", salt, iterations },
    baseKey,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"]
  );
}

/** 用口令加密任意可序列化对象，返回信封 JSON 字符串。 */
export async function encryptJson(
  obj: unknown,
  password: string,
  type: EncryptedEnvelopeType = "connections-export"
): Promise<string> {
  const salt = crypto.getRandomValues(new Uint8Array(SALT_BYTES));
  const iv = crypto.getRandomValues(new Uint8Array(IV_BYTES));
  const key = await deriveKey(password, salt, PBKDF2_ITERATIONS);
  const plaintext = new TextEncoder().encode(JSON.stringify(obj));
  const cipher = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, plaintext);

  const envelope: EncryptedEnvelope = {
    app: "STerm",
    type,
    version: 1,
    kdf: { algo: "PBKDF2", hash: "SHA-256", iterations: PBKDF2_ITERATIONS, salt: toBase64(salt) },
    cipher: "AES-GCM",
    iv: toBase64(iv),
    data: toBase64(new Uint8Array(cipher)),
  };
  return JSON.stringify(envelope, null, 2);
}

/** 解析并用口令解密信封 JSON。口令错误或文件损坏时抛错。 */
export async function decryptJson<T = unknown>(
  text: string,
  password: string,
  expectedType: EncryptedEnvelopeType = "connections-export"
): Promise<T> {
  let env: EncryptedEnvelope;
  try {
    env = JSON.parse(text);
  } catch {
    throw new Error("文件格式无效：不是合法的 JSON");
  }
  if (env?.app !== "STerm" || env?.type !== expectedType) {
    throw new Error(
      expectedType === "connections-export"
        ? "文件格式无效：不是 STerm 导出文件"
        : "本地机器列表格式无效"
    );
  }
  const salt = fromBase64(env.kdf.salt);
  const iv = fromBase64(env.iv);
  const key = await deriveKey(password, salt, env.kdf.iterations ?? PBKDF2_ITERATIONS);
  let plain: ArrayBuffer;
  try {
    plain = await crypto.subtle.decrypt(
      { name: "AES-GCM", iv },
      key,
      fromBase64(env.data)
    );
  } catch {
    throw new Error("解密失败：口令错误或文件已损坏");
  }
  return JSON.parse(new TextDecoder().decode(plain)) as T;
}
