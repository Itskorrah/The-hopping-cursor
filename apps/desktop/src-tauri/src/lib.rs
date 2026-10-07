use hop_core::{LayoutDisplay, Service, Settings, Snapshot};
use std::{path::PathBuf, sync::Arc};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State,
};
type Engine = Arc<Service>;
#[tauri::command]
fn save_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine.save_profile(&name).map_err(|e| e.to_string())
}
#[tauri::command]
fn load_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine.load_profile(&name).map_err(|e| e.to_string())
}
#[tauri::command]
fn delete_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine.delete_profile(&name).map_err(|e| e.to_string())
}
#[tauri::command]
fn snapshot(engine: State<'_, Engine>) -> Snapshot {
    engine.snapshot()
}
#[tauri::command]
fn update_settings(engine: State<'_, Engine>, settings: Settings) -> Result<(), String> {
    engine.settings(settings).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_layout(engine: State<'_, Engine>, layout: Vec<LayoutDisplay>) -> Result<(), String> {
    engine.set_layout(layout).map_err(|e| e.to_string())
}
#[tauri::command]
fn approve_peer(engine: State<'_, Engine>, id: String, public_key: String) -> Result<(), String> {
    engine
        .approve_peer(&id, &public_key)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn forget_peer(engine: State<'_, Engine>, id: String) -> Result<(), String> {
    engine.forget_peer(&id).map_err(|e| e.to_string())
}
#[tauri::command]
fn peer_permissions(
    engine: State<'_, Engine>,
    id: String,
    input: bool,
    clipboard: bool,
    files: bool,
) -> Result<(), String> {
    engine
        .peer_permissions(&id, input, clipboard, files)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn connect_address(engine: State<'_, Engine>, address: String) -> Result<(), String> {
    engine
        .inner()
        .connect(&address)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn send_files(
    engine: State<'_, Engine>,
    peer: String,
    paths: Vec<String>,
) -> Result<(), String> {
    if paths.len() > 100 {
        return Err("Select at most 100 files at once".into());
    }
    let engine = engine.inner().clone();
    for path in paths {
        engine
            .send_file(&peer, PathBuf::from(path))
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
fn respond_transfer(engine: State<'_, Engine>, id: String, accepted: bool) -> Result<(), String> {
    engine
        .respond_transfer(&id, accepted)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn cancel_transfer(engine: State<'_, Engine>, id: String) -> Result<(), String> {
    engine.cancel_transfer(&id).map_err(|e| e.to_string())
}
#[tauri::command]
fn release_input(engine: State<'_, Engine>) {
    engine.release()
}
#[tauri::command]
fn export_diagnostics(engine: State<'_, Engine>) -> String {
    engine.export_diagnostics()
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let app=tauri::Builder::default().plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![snapshot,update_settings,save_layout,approve_peer,forget_peer,peer_permissions,connect_address,send_files,respond_transfer,cancel_transfer,release_input,export_diagnostics,save_profile,load_profile,delete_profile])
        .setup(|app|{
            let engine=tauri::async_runtime::block_on(Service::start(hop_core::config::default_config_dir()?,None,hop_core::platform::native()))?;app.manage(engine);
            let open=MenuItem::with_id(app,"open","Open The Hopping Cursor",true,None::<&str>)?;
            let pause=MenuItem::with_id(app,"pause","Pause / Resume sharing",true,None::<&str>)?;
            let release=MenuItem::with_id(app,"release","Bring input back here",true,None::<&str>)?;
            let quit=MenuItem::with_id(app,"quit","Quit",true,None::<&str>)?;
            let menu=Menu::with_items(app,&[&open,&pause,&release,&quit])?;
            let mut tray=TrayIconBuilder::new().menu(&menu).tooltip("The Hopping Cursor").on_menu_event(|app,event|{
                let engine=app.state::<Engine>();match event.id.as_ref(){
                    "open"=>if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.set_focus();},
                    "pause"=>{let mut settings=engine.snapshot().settings;settings.paused=!settings.paused;let _=engine.settings(settings);},
                    "release"=>engine.release(),"quit"=>{engine.shutdown();app.exit(0);},_=>{}
                }
            });if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}tray.build(app)?;Ok(())
        })
        .on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{api.prevent_close();let _=window.hide();}})
        .build(tauri::generate_context!()).expect("Could not start The Hopping Cursor. Check permissions and whether TCP port 24810 is already in use.");
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(engine) = app.try_state::<Engine>() {
                engine.shutdown();
            }
        }
    });
}
