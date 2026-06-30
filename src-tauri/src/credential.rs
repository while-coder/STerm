// 系统凭证中的主密码读写。Windows 使用 Credential Manager；其他平台后续可接
// Keychain / Secret Service。

#[tauri::command]
pub fn get_master_password() -> Result<Option<String>, String> {
    platform::get_master_password()
}

#[tauri::command]
pub fn set_master_password(password: String) -> Result<(), String> {
    if password.is_empty() {
        return Err("主密码不能为空".to_string());
    }
    platform::set_master_password(password)
}

#[tauri::command]
pub fn delete_master_password() -> Result<(), String> {
    platform::delete_master_password()
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

    const TARGET_NAME: &str = "STerm:master-password";
    const USER_NAME: &str = "STerm";

    fn wide_null(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error(action: &str) -> String {
        format!("{}失败：Windows 错误 {}", action, unsafe { GetLastError() })
    }

    pub fn get_master_password() -> Result<Option<String>, String> {
        let target = wide_null(TARGET_NAME);
        let mut credential: *mut CREDENTIALW = null_mut();
        let ok = unsafe {
            CredReadW(
                target.as_ptr(),
                CRED_TYPE_GENERIC,
                0,
                &mut credential,
            )
        };

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
                std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                )
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

    pub fn set_master_password(password: String) -> Result<(), String> {
        let target = wide_null(TARGET_NAME);
        let username = wide_null(USER_NAME);
        let mut blob = password.into_bytes();
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

    pub fn delete_master_password() -> Result<(), String> {
        let target = wide_null(TARGET_NAME);
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

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn get_master_password() -> Result<Option<String>, String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }

    pub fn set_master_password(_password: String) -> Result<(), String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }

    pub fn delete_master_password() -> Result<(), String> {
        Err("当前平台暂未支持系统凭证".to_string())
    }
}
