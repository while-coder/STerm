mod credential;
mod gist;
mod portab;
mod sftp;
mod ssh;
mod state;

use state::AppState;
use tauri_plugin_log::{Target, TargetKind, TimezoneStrategy};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // 日志插件需最先注册，以便捕获其余插件与命令的日志。
        // 同时输出到标准输出与应用日志目录文件（前端日志经 plugin-log 转发至此）。
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("sterm".into()),
                    }),
                ])
                // 单文件上限 5MB，超出后滚动并保留历史文件。
                .max_file_size(5_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                // 使用本地时区，便于排查问题时与系统时间对齐。
                .timezone_strategy(TimezoneStrategy::UseLocal)
                // 开发期记录到 Trace，发布版仅记录 Info 及以上。
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Trace
                } else {
                    log::LevelFilter::Info
                })
                .build(),
        )
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
            sftp::sftp_create_file,
            sftp::sftp_rename,
            sftp::sftp_remove,
            sftp::ensure_dir,
            sftp::open_dir,
            sftp::open_local_path,
            portab::read_text_file,
            portab::write_text_file,
            portab::import_private_key,
            credential::get_master_password,
            credential::set_master_password,
            credential::delete_master_password,
            credential::set_credential,
            credential::delete_credential,
            gist::gist_validate,
            gist::gist_pull,
            gist::gist_push,
            gist::gist_find,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
