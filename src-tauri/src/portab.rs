// 机器列表导入 / 导出的文件读写：读私钥 / 读导入文件、写导出文件、导入私钥回写到磁盘。
// 加解密在前端完成（Web Crypto），后端只负责落盘；私钥回写时在 Unix 下收紧权限到 600。

use std::fs;
use std::path::PathBuf;

use crate::ssh::{expand_home_path, home_dir};

/// 读取文本文件（支持 `~` 展开）；用于读私钥内容和读导入文件。
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let p = expand_home_path(&path);
    fs::read_to_string(&p).map_err(|e| format!("读取文件失败（{}）：{e}", p.display()))
}

/// 写文本文件（支持 `~` 展开）；用于写导出文件。
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    let p = expand_home_path(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败（{}）：{e}", parent.display()))?;
    }
    fs::write(&p, contents).map_err(|e| format!("写入文件失败（{}）：{e}", p.display()))
}

/// 把导入的私钥内容写到 `~/.ssh/sterm-keys/<id>`，返回该路径供 privateKeyPath 指向。
/// Unix 下将权限收紧到 600，避免 ssh 因权限过宽拒绝加载私钥。
#[tauri::command]
pub fn import_private_key(id: String, contents: String) -> Result<String, String> {
    // 防止 id 中的路径分隔符越权到其他目录。
    let safe_id: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe_id.is_empty() {
        return Err("无效的私钥标识".into());
    }

    let home = home_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    let dir: PathBuf = home.join(".ssh").join("sterm-keys");
    fs::create_dir_all(&dir).map_err(|e| format!("创建私钥目录失败（{}）：{e}", dir.display()))?;

    let key_path = dir.join(&safe_id);
    fs::write(&key_path, contents)
        .map_err(|e| format!("写入私钥失败（{}）：{e}", key_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&key_path, perms)
            .map_err(|e| format!("设置私钥权限失败（{}）：{e}", key_path.display()))?;
    }

    Ok(key_path.to_string_lossy().into_owned())
}
