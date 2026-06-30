mod portab;
mod credential;
mod sftp;
mod ssh;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    // 应用内更新：仅桌面端注册 updater / process 插件（移动端不支持）。
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            ssh::default_private_key_path,
            ssh::ssh_connect,
            ssh::ssh_write,
            ssh::ssh_resize,
            ssh::ssh_disconnect,
            sftp::sftp_home,
            sftp::sftp_list,
            sftp::sftp_download,
            sftp::sftp_download_dir,
            sftp::sftp_upload,
            sftp::sftp_cancel,
            sftp::sftp_mkdir,
            sftp::sftp_rename,
            sftp::sftp_remove,
            sftp::ensure_dir,
            portab::read_text_file,
            portab::write_text_file,
            portab::import_private_key,
            credential::get_master_password,
            credential::set_master_password,
            credential::delete_master_password,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
