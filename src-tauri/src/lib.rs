mod auth;
mod config;
mod error;
mod instances;
mod java;
mod minecraft;
mod modrinth;
mod news;
mod notify;
mod paths;
mod skins;
mod state;
mod yggdrasil;

use minecraft::{launch, loaders, manifest};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state::AppState::default())
        .setup(|_| {
            // Servidor de skins local para cuentas no-premium (skinfix)
            tauri::async_runtime::spawn(async {
                if let Err(e) = yggdrasil::start().await {
                    eprintln!("[KIRMC] no se pudo iniciar el servidor de skins: {e}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            yggdrasil::set_offline_skin,
            yggdrasil::set_offline_cape,
            // Configuración
            config::get_config,
            config::save_config,
            config::data_dir,
            // Skins, capas y biblioteca
            skins::lookup_player,
            skins::fetch_texture,
            skins::mineskin_gallery,
            skins::lookup_capes,
            skins::elyby_skin,
            skins::list_skins,
            skins::save_skin,
            skins::read_skin,
            skins::delete_skin,
            // Modrinth
            modrinth::modrinth_search,
            modrinth::modrinth_versions,
            modrinth::modrinth_install,
            modrinth::modrinth_install_modpack,
            // Cuentas (Premium + No-Premium)
            auth::list_accounts,
            auth::add_offline_account,
            auth::remove_account,
            auth::ms_login_window,
            auth::upload_skin,
            auth::set_cape,
            // Versiones y cargadores
            manifest::list_versions,
            loaders::list_loader_versions,
            // Instancias
            instances::list_instances,
            instances::create_instance,
            instances::update_instance,
            instances::delete_instance,
            instances::instance_dir,
            instances::list_mods,
            instances::toggle_mod,
            // Juego
            launch::install_instance,
            launch::launch_instance,
            launch::kill_instance,
            launch::running_instances,
            // Varios
            java::detect_java,
            news::fetch_news,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar KIRMC Launcher");
}
