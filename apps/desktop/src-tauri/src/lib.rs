use hop_core::{LayoutDisplay, Service, Settings, Snapshot};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State,
};
#[derive(Clone, Default)]
struct Engine(Arc<Mutex<EngineState>>);
#[derive(Default)]
struct EngineState {
    service: Option<Arc<Service>>,
    error: Option<String>,
    stopping: bool,
}
impl Engine {
    fn ready(&self) -> Result<Arc<Service>, String> {
        let state = self.0.lock().unwrap();
        if state.stopping {
            return Err("The application is stopping".into());
        }
        state.service.clone().ok_or_else(|| state.error.clone().unwrap_or_else(||
            "Starting securely. If macOS requests access to your existing pairing identity in Keychain, approve that system dialog. Sharing stays unavailable until startup completes.".into()))
    }
    fn shutdown(&self) {
        let mut state = self.0.lock().unwrap();
        state.stopping = true;
        if let Some(service) = state.service.take() {
            service.shutdown();
        }
    }
    fn start(&self) -> std::io::Result<()> {
        let engine = self.clone();
        std::thread::Builder::new().name("hop-startup".into()).spawn(move || {
            // Protected storage can wait for a human OS dialog. Keep it off the
            // webview thread and never rotate identity or fall back on failure.
            let result = tauri::async_runtime::block_on(async {
                Service::start(hop_core::config::default_config_dir()?, None, hop_core::platform::native()).await
            });
            let mut state = engine.0.lock().unwrap();
            match result {
                Ok(service) if state.stopping => service.shutdown(),
                Ok(service) => state.service = Some(service),
                Err(error) => state.error = Some(format!("Secure startup could not complete: {error}. Check the system approval dialog, then quit and reopen. Your pairing identity has not been replaced.")),
            }
        })?;
        Ok(())
    }
}
#[tauri::command]
fn set_hop_key(
    engine: State<'_, Engine>,
    device_id: String,
    display_id: String,
    keys: Vec<u16>,
) -> Result<(), String> {
    engine
        .ready()?
        .set_hop_key(hop_core::HopKey {
            device_id,
            display_id,
            keys,
        })
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn jump_to_display(
    engine: State<'_, Engine>,
    device_id: String,
    display_id: String,
) -> Result<(), String> {
    engine
        .ready()?
        .jump_to_display(&device_id, &display_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine
        .ready()?
        .save_profile(&name)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn load_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine
        .ready()?
        .load_profile(&name)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn delete_profile(engine: State<'_, Engine>, name: String) -> Result<(), String> {
    engine
        .ready()?
        .delete_profile(&name)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn request_input_permissions(engine: State<'_, Engine>) -> Result<hop_core::Permissions, String> {
    engine
        .ready()?
        .request_input_permissions()
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn snapshot(engine: State<'_, Engine>) -> Result<Snapshot, String> {
    Ok(engine.ready()?.snapshot())
}
#[tauri::command]
fn update_settings(engine: State<'_, Engine>, settings: Settings) -> Result<(), String> {
    engine
        .ready()?
        .settings(settings)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn save_layout(engine: State<'_, Engine>, layout: Vec<LayoutDisplay>) -> Result<(), String> {
    engine
        .ready()?
        .set_layout(layout)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn share_layout(engine: State<'_, Engine>) -> Result<usize, String> {
    engine.ready()?.share_layout().map_err(|e| e.to_string())
}
#[tauri::command]
fn accept_layout(engine: State<'_, Engine>, peer: String) -> Result<(), String> {
    engine
        .ready()?
        .accept_layout(&peer)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn dismiss_layout(engine: State<'_, Engine>, peer: String) -> Result<(), String> {
    engine.ready()?.dismiss_layout(&peer);
    Ok(())
}
#[tauri::command]
fn approve_peer(engine: State<'_, Engine>, id: String, public_key: String) -> Result<(), String> {
    engine
        .ready()?
        .approve_peer(&id, &public_key)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn forget_peer(engine: State<'_, Engine>, id: String) -> Result<(), String> {
    engine.ready()?.forget_peer(&id).map_err(|e| e.to_string())
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
        .ready()?
        .peer_permissions(&id, input, clipboard, files)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn connect_address(engine: State<'_, Engine>, address: String) -> Result<(), String> {
    engine
        .ready()?
        .connect(&address)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn send_copied_files(engine: State<'_, Engine>, peer: String) -> Result<(), String> {
    engine
        .ready()?
        .send_copied_files(&peer)
        .await
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn copy_received_file(engine: State<'_, Engine>, id: String) -> Result<(), String> {
    engine
        .ready()?
        .copy_received_file(&id)
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
    let engine = engine.ready()?;
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
        .ready()?
        .respond_transfer(&id, accepted)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn cancel_transfer(engine: State<'_, Engine>, id: String) -> Result<(), String> {
    engine
        .ready()?
        .cancel_transfer(&id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn release_input(engine: State<'_, Engine>) -> Result<(), String> {
    engine.ready()?.release();
    Ok(())
}
#[tauri::command]
fn export_diagnostics(engine: State<'_, Engine>) -> Result<String, String> {
    Ok(engine.ready()?.export_diagnostics())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle, engine: State<'_, Engine>) {
    engine.shutdown();
    app.exit(0);
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let app=tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![quit_app,share_layout,accept_layout,dismiss_layout,send_copied_files,copy_received_file,request_input_permissions,snapshot,update_settings,save_layout,approve_peer,forget_peer,peer_permissions,connect_address,send_files,respond_transfer,cancel_transfer,release_input,export_diagnostics,save_profile,load_profile,delete_profile,set_hop_key,jump_to_display])
        .setup(|app|{
            let engine = Engine::default(); app.manage(engine.clone());
            let open=MenuItem::with_id(app,"open","Open The Hopping Cursor",true,None::<&str>)?;
            let pause=MenuItem::with_id(app,"pause","Pause / Resume sharing",true,None::<&str>)?;
            let release=MenuItem::with_id(app,"release","Bring input back here",true,None::<&str>)?;
            let quit=MenuItem::with_id(app,"quit","Quit",true,None::<&str>)?;
            let menu=Menu::with_items(app,&[&open,&pause,&release,&quit])?;
            let mut tray=TrayIconBuilder::new().menu(&menu).tooltip("The Hopping Cursor").on_menu_event(|app,event|{
                let engine=app.state::<Engine>();match event.id.as_ref(){
                    "open"=>if let Some(w)=app.get_webview_window("main"){let _=w.show();let _=w.set_focus();},
                    "pause"=>{if let Ok(service) = engine.ready() { let mut settings=service.snapshot().settings;settings.paused = !settings.paused;let _=service.settings(settings); }},
                    "release"=>{if let Ok(service) = engine.ready() { service.release(); }},"quit"=>{engine.shutdown();app.exit(0);},_=>{}
                }
            });if let Some(icon)=app.default_window_icon(){tray=tray.icon(icon.clone());}tray.build(app)?;engine.start()?;Ok(())
        })
        .on_window_event(|window,event|{if let tauri::WindowEvent::CloseRequested{api,..}=event{api.prevent_close();let _=window.hide();}})
        .build(tauri::generate_context!()).expect("Could not start The Hopping Cursor. Check permissions and whether TCP port 24810 is already in use.");
    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        if let tauri::RunEvent::Exit = event {
            if let Some(engine) = app.try_state::<Engine>() {
                engine.shutdown();
            }
        }
    });
}
