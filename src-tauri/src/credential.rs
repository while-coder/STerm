// 系统凭证读写：按 key 存取一条秘密（主密码、GitHub PAT 等）。
// - Windows: Credential Manager（target = "STerm:<key>"）
// - macOS: Keychain（security 命令，service = "STerm:<key>"）
// - Linux: Secret Service（secret-tool/libsecret，service = "STerm", account = <key>）

const MASTER_PASSWORD_KEY: &str = "master-password";

/// 供其他后端模块（如 gist）按 key 读取秘密，不经过前端。
pub(crate) fn read_credential(key: &str) -> Result<Option<String>, String> {
    platform::get(key)
}

// —— 通用按 key 读写（出于安全考虑不提供读回前端的命令，仅写入/删除）——

#[tauri::command]
pub fn set_credential(key: String, value: String) -> Result<(), String> {
    if value.is_empty() {
        return Err("凭证内容不能为空".to_string());
    }
    platform::set(&key, value)
}

#[tauri::command]
pub fn delete_credential(key: String) -> Result<(), String> {
    platform::delete(&key)
}

// —— 主密码（保持原有前端 API 不变，内部转调通用实现）——

#[tauri::command]
pub fn get_master_password() -> Result<Option<String>, String> {
    platform::get(MASTER_PASSWORD_KEY)
}

#[tauri::command]
pub fn set_master_password(password: String) -> Result<(), String> {
    if password.is_empty() {
        return Err("主密码不能为空".to_string());
    }
    platform::set(MASTER_PASSWORD_KEY, password)
}

#[tauri::command]
pub fn delete_master_password() -> Result<(), String> {
    platform::delete(MASTER_PASSWORD_KEY)
}

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    const USER_NAME: &str = "STerm";

    fn target_name(key: &str) -> String {
        format!("STerm:{key}")
    }

    fn wide_null(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error(action: &str) -> String {
        format!("{}失败：Windows 错误 {}", action, unsafe {
            GetLastError()
        })
    }

    pub fn get(key: &str) -> Result<Option<String>, String> {
        let target = wide_null(&target_name(key));
        let mut credential: *mut CREDENTIALW = null_mut();
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };

        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(last_error("读取系统凭证"));
        }

        if credential.is_null() {
            return Ok(None);
        }

        let cred = unsafe { &*credential };
        let bytes = if cred.CredentialBlob.is_null() || cred.CredentialBlobSize == 0 {
            Vec::new()
        } else {
            unsafe {
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize)
                    .to_vec()
            }
        };
        unsafe { CredFree(credential as *const c_void) };

        if bytes.is_empty() {
            return Ok(None);
        }
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "系统凭证内容不是有效的 UTF-8".to_string())
    }

    pub fn set(key: &str, value: String) -> Result<(), String> {
        let target = wide_null(&target_name(key));
        let username = wide_null(USER_NAME);
        let mut blob = value.into_bytes();
        let mut credential = unsafe { std::mem::zeroed::<CREDENTIALW>() };

        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target.as_ptr() as *mut u16;
        credential.CredentialBlobSize = blob.len() as u32;
        credential.CredentialBlob = blob.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        credential.UserName = username.as_ptr() as *mut u16;

        let ok = unsafe { CredWriteW(&credential, 0) };
        if ok == 0 {
            return Err(last_error("保存系统凭证"));
        }
        Ok(())
    }

    pub fn delete(key: &str) -> Result<(), String> {
        let target = wide_null(&target_name(key));
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(());
            }
            return Err(last_error("删除系统凭证"));
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    const ACCOUNT: &str = "STerm";

    fn service(key: &str) -> String {
        format!("STerm:{key}")
    }

    fn strip_trailing_newline(mut s: String) -> String {
        while s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        s
    }

    fn output_text(bytes: Vec<u8>) -> String {
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn not_found(stderr: &str) -> bool {
        stderr.contains("could not be found")
            || stderr.contains("The specified item could not be found")
    }

    pub fn get(key: &str) -> Result<Option<String>, String> {
        let output = Command::new("security")
            .args(["find-generic-password", "-a", ACCOUNT, "-s", &service(key), "-w"])
            .output()
            .map_err(|e| format!("读取 Keychain 失败：{e}"))?;
        if output.status.success() {
            return Ok(Some(strip_trailing_newline(output_text(output.stdout))));
        }
        let stderr = output_text(output.stderr);
        if not_found(&stderr) {
            return Ok(None);
        }
        Err(format!("读取 Keychain 失败：{stderr}"))
    }

    pub fn set(key: &str, value: String) -> Result<(), String> {
        let output = Command::new("security")
            .args([
                "add-generic-password",
                "-a",
                ACCOUNT,
                "-s",
                &service(key),
                "-w",
                value.as_str(),
                "-U",
            ])
            .output()
            .map_err(|e| format!("保存 Keychain 失败：{e}"))?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!("保存 Keychain 失败：{}", output_text(output.stderr)))
    }

    pub fn delete(key: &str) -> Result<(), String> {
        let output = Command::new("security")
            .args(["delete-generic-password", "-a", ACCOUNT, "-s", &service(key)])
            .output()
            .map_err(|e| format!("删除 Keychain 凭证失败：{e}"))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = output_text(output.stderr);
        if not_found(&stderr) {
            return Ok(());
        }
        Err(format!("删除 Keychain 凭证失败：{stderr}"))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::io::Write;
    use std::process::{Command, Stdio};

    const SERVICE: &str = "STerm";

    fn strip_trailing_newline(mut s: String) -> String {
        while s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        s
    }

    fn output_text(bytes: Vec<u8>) -> String {
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn secret_tool_hint() -> String {
        "Linux 系统凭证需要 Secret Service 和 secret-tool（通常由 libsecret 提供）".to_string()
    }

    fn run_secret_tool(args: &[&str]) -> Result<std::process::Output, String> {
        Command::new("secret-tool")
            .args(args)
            .output()
            .map_err(|e| format!("{}：{e}", secret_tool_hint()))
    }

    pub fn get(key: &str) -> Result<Option<String>, String> {
        let output = run_secret_tool(&["lookup", "service", SERVICE, "account", key])?;
        if output.status.success() {
            let secret = strip_trailing_newline(output_text(output.stdout));
            return if secret.is_empty() {
                Ok(None)
            } else {
                Ok(Some(secret))
            };
        }
        let stderr = output_text(output.stderr);
        if stderr.trim().is_empty() || stderr.contains("No such item") {
            return Ok(None);
        }
        Err(format!("读取 Secret Service 凭证失败：{stderr}"))
    }

    pub fn set(key: &str, value: String) -> Result<(), String> {
        let label = format!("STerm {key}");
        let mut child = Command::new("secret-tool")
            .args([
                "store", "--label", &label, "service", SERVICE, "account", key,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{}：{e}", secret_tool_hint()))?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| "写入 Secret Service 凭证失败：无法打开 stdin".to_string())?;
            stdin
                .write_all(value.as_bytes())
                .map_err(|e| format!("写入 Secret Service 凭证失败：{e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("保存 Secret Service 凭证失败：{e}"))?;
        if output.status.success() {
            return Ok(());
        }
        Err(format!(
            "保存 Secret Service 凭证失败：{}",
            output_text(output.stderr)
        ))
    }

    pub fn delete(key: &str) -> Result<(), String> {
        let output = run_secret_tool(&["clear", "service", SERVICE, "account", key])?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = output_text(output.stderr);
        if stderr.trim().is_empty() || stderr.contains("No such item") {
            return Ok(());
        }
        Err(format!("删除 Secret Service 凭证失败：{stderr}"))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod platform {
    pub fn get(_key: &str) -> Result<Option<String>, String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }

    pub fn set(_key: &str, _value: String) -> Result<(), String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }

    pub fn delete(_key: &str) -> Result<(), String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }
}
