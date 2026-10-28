//! Notificaciones de escritorio (respetan el ajuste `notifications` y el idioma).

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::config;

pub enum Note<'a> {
    InstallDone(&'a str),
    ModsInstalled(usize),
    GameCrashed(&'a str, i32),
}

fn text(lang: &str, note: &Note) -> (String, String) {
    match (lang, note) {
        ("en", Note::InstallDone(n)) => ("Instance ready".into(), format!("{n} is installed and ready to play")),
        ("ja", Note::InstallDone(n)) => ("インスタンスの準備完了".into(), format!("{n} をプレイできます")),
        (_, Note::InstallDone(n)) => ("Instancia lista".into(), format!("{n} está instalada y lista para jugar")),
        ("en", Note::ModsInstalled(c)) => ("Mods installed".into(), format!("{c} file(s) downloaded")),
        ("ja", Note::ModsInstalled(c)) => ("Mod をインストールしました".into(), format!("{c} 個のファイル")),
        (_, Note::ModsInstalled(c)) => ("Mods instalados".into(), format!("{c} archivo(s) descargado(s)")),
        ("en", Note::GameCrashed(n, c)) => ("Minecraft crashed".into(), format!("{n} exited with code {c}. Check the logs.")),
        ("ja", Note::GameCrashed(n, c)) => ("Minecraft がクラッシュしました".into(), format!("{n} (コード {c})。ログを確認してください")),
        (_, Note::GameCrashed(n, c)) => ("Minecraft se cerró con error".into(), format!("{n} terminó con código {c}. Revisa los logs.")),
    }
}

pub fn send(app: &AppHandle, note: Note) {
    let cfg = config::load();
    if !cfg.notifications {
        return;
    }
    let (title, body) = text(&cfg.language, &note);
    let _ = app.notification().builder().title(title).body(body).show();
}
