#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use clap::Parser;
use commands::AppState;
use std::sync::Mutex;
use tauri::Emitter;

#[derive(Parser)]
#[command(name = "a2l-editor")]
struct Cli {
    /// 自动加载数据包路径
    #[arg(short, long)]
    package: Option<String>,
    /// 自动加载 A2L 文件路径
    #[arg(short, long)]
    a2l: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let mut package_path = cli.package;
    let mut a2l_path = cli.a2l;
    
    // 如果命令行参数为空，尝试从环境变量读取
    if package_path.is_none() {
        package_path = std::env::var("A2L_TEST_PACKAGE").ok();
    }
    if a2l_path.is_none() {
        a2l_path = std::env::var("A2L_TEST_A2L").ok();
    }
    
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(Mutex::new(AppState::default()))
        .setup(move |app| {
            // 如果有启动参数或环境变量，发送事件到前端
            if package_path.is_some() || a2l_path.is_some() {
                app.handle().emit("auto-load-files", serde_json::json!({
                    "package": package_path,
                    "a2l": a2l_path
                })).ok();
            }
            
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_elf,
            commands::load_package,
            commands::generate_package,
            commands::load_a2l,
            commands::search_elf_entries,
            commands::get_elf_count,
            commands::search_a2l_variables,
            commands::export_entries,
            commands::delete_variables,
            commands::save_a2l_changes,
            commands::set_endianness,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
