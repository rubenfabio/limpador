//! Desktop shell: wires the core library to the Tauri window. No business rules here (ADR-0003).

pub mod app_state;
pub mod sandbox;
pub mod sandbox_commands;
pub mod view;

use app_state::{AppState, Choices, Paths, Settings};
use limpador_core::adapters::SystemCommandRunner;
use limpador_core::message::Message;
use limpador_core::onboarding::RootSuggestion;
use limpador_core::ports::CommandRunner;
use sandbox::{SANDBOX_ENV, SandboxDecision};
use sandbox_commands::SandboxCommands;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use view::{AnalysisView, progress_event};

#[tauri::command]
fn core_version() -> &'static str {
    limpador_core::version()
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app (J1)
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app (J1)
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
async fn suggest_roots(state: State<'_, AppState>) -> Result<Vec<RootSuggestion>, String> {
    Ok(state.suggest_roots())
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app (J1)
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn validate_root(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state.validate_root(&path)
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app (J1)
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn complete_onboarding(state: State<'_, AppState>, choices: Choices) -> Result<Settings, String> {
    state.complete_onboarding(choices)
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app (J1)
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn skip_onboarding(state: State<'_, AppState>) -> Result<Settings, String> {
    state.skip_onboarding()
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e da UI
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
async fn start_docker(state: State<'_, AppState>) -> Result<(), Message> {
    state.start_docker()
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
async fn analyze(app: AppHandle, state: State<'_, AppState>) -> Result<AnalysisView, String> {
    let items_app = app.clone();
    Ok(state.analyze(
        Arc::new(move |collector, s| {
            // A closed window just misses progress; the result still comes back.
            let _ = app.emit("analysis-progress", progress_event(collector, s));
        }),
        Arc::new(move |event| {
            let _ = items_app.emit("analysis-items", event);
        }),
    ))
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn last_analysis(state: State<'_, AppState>) -> Option<AnalysisView> {
    state.last_analysis()
}

// motivo: casca fina que só repassa ao AppState (testado); a ligação é coberta pelo e2e do app
#[cfg_attr(test, mutants::skip)]
#[tauri::command]
fn cancel_analysis(state: State<'_, AppState>) {
    state.cancel_analysis();
}

/// `%LOCALAPPDATA%`, where Docker Desktop and WSL keep their virtual disks.
fn local_app_data(home: &std::path::Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Local"))
}

// motivo: só abre a janela e liga as peças; as decisões estão em `sandbox` e `app_state`
// (com testes) e a abertura real é coberta pelo e2e do app (T-00.04).
#[cfg_attr(test, mutants::skip)]
pub fn run() {
    let sandbox = sandbox::decide(std::env::var_os(SANDBOX_ENV));
    if let Some(message) = sandbox::refusal_message(&sandbox) {
        eprintln!("{message}");
        std::process::exit(2);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // The main window is created here (not by tauri.conf.json) so that a sandboxed
        // run can keep the webview profile inside the fake machine.
        .setup(move |app| {
            let (paths, commands): (Paths, Arc<dyn CommandRunner>) = match &sandbox {
                SandboxDecision::Enabled(root) => {
                    (Paths::sandbox(root), Arc::new(SandboxCommands::new(root)))
                }
                _ => {
                    let home = app.path().home_dir()?;
                    let local = local_app_data(&home);
                    (
                        Paths::real(app.path().app_config_dir()?, home, local),
                        Arc::new(SystemCommandRunner),
                    )
                }
            };
            app.manage(AppState::load(paths, commands));

            let config = app
                .config()
                .app
                .windows
                .first()
                .ok_or("main window missing from tauri.conf.json")?
                .clone();
            let mut window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?;
            if let SandboxDecision::Enabled(root) = &sandbox {
                window = window.data_directory(sandbox::webview_data_dir(root));
            }
            window.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_version,
            get_settings,
            suggest_roots,
            validate_root,
            complete_onboarding,
            skip_onboarding,
            start_docker,
            analyze,
            last_analysis,
            cancel_analysis
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Limpador");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t00_01_ca1_app_command_delegates_to_core() {
        assert_eq!(core_version(), limpador_core::version());
    }

    #[test]
    fn local_app_data_falls_back_to_the_home_folder() {
        let home = std::path::Path::new("C:\\Users\\dev");
        let expected = std::env::var_os("LOCALAPPDATA")
            .map_or_else(|| home.join("AppData").join("Local"), PathBuf::from);
        assert_eq!(local_app_data(home), expected);
    }
}
