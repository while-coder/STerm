// 主密码生命周期：系统凭证读写、首次设置、修改主密码，并驱动机器列表加密存储。
import { ref } from "vue";
import { getMasterPassword, setMasterPassword } from "../api";
import { useConnections } from "./useConnections";

const initialized = ref(false);
const busy = ref(false);
const needsSetup = ref(false);
const error = ref("");
const masterPassword = ref("");

function toMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export function useSecurity() {
  const { initConnections, reencryptConnections, resetConnections } = useConnections();

  async function initSecurity() {
    if (busy.value || initialized.value) return;
    busy.value = true;
    error.value = "";
    try {
      const saved = await getMasterPassword();
      if (!saved) {
        needsSetup.value = true;
        resetConnections();
      } else {
        masterPassword.value = saved;
        needsSetup.value = false;
        await initConnections(saved);
      }
    } catch (e) {
      error.value = toMessage(e);
      needsSetup.value = true;
      resetConnections();
    } finally {
      initialized.value = true;
      busy.value = false;
    }
  }

  async function createMasterPassword(password: string) {
    if (!password) throw new Error("主密码不能为空");
    busy.value = true;
    error.value = "";
    try {
      await setMasterPassword(password);
      masterPassword.value = password;
      needsSetup.value = false;
      await initConnections(password, true);
    } catch (e) {
      error.value = toMessage(e);
      throw e;
    } finally {
      busy.value = false;
      initialized.value = true;
    }
  }

  async function changeMasterPassword(password: string) {
    if (!masterPassword.value) throw new Error("当前未解锁，无法修改主密码");
    if (!password) throw new Error("主密码不能为空");
    const previous = masterPassword.value;
    busy.value = true;
    error.value = "";
    try {
      await reencryptConnections(password);
      await setMasterPassword(password);
      masterPassword.value = password;
    } catch (e) {
      await reencryptConnections(previous).catch(() => undefined);
      error.value = toMessage(e);
      throw e;
    } finally {
      busy.value = false;
    }
  }

  function requireMasterPassword(): string {
    if (!masterPassword.value) throw new Error("主密码尚未初始化");
    return masterPassword.value;
  }

  return {
    initialized,
    busy,
    needsSetup,
    error,
    masterPassword,
    initSecurity,
    createMasterPassword,
    changeMasterPassword,
    requireMasterPassword,
  };
}
