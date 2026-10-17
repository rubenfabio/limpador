//! Desktop shell: wires the core library to the Tauri window. No business rules here (ADR-0003).

pub mod app_state;
pub mod sandbox;

use app_state::{AnalysisSummary, AppState, Choices, Paths, Settings};
use limpador_core::collector::Registry;
use limpador_core::onboarding::RootSuggestion;
use sandbox::{SANDBOX_ENV, SandboxDecision};
use tauri::{Manager, State};

#[tauri::command]
fn core_version() -> &'static str {
    limpador_core::version()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
async fn suggest_roots(state: State<'_, AppState>) -> Result<Vec<RootSuggestion>, String> {
    Ok(state.suggest_roots())
}

#[tauri::command]
fn validate_root(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state.validate_root(&path)
}

#[tauri::command]
fn complete_onboarding(state: State<'_, AppState>, choices: Choices) -> Result<Settings, String> {
    state.complete_onboarding(choices)
}

#[tauri::command]
fn skip_onboarding(state: State<'_, AppState>) -> Result<Settings, String> {
    state.skip_onboarding()
}

#[tauri::command]
async fn run_analysis(state: State<'_, AppState>) -> Result<AnalysisSummary, String> {
    Ok(state.run_analysis())
}

/// Collectors arrive with F03–F07; the engine already runs an empty registry.
fn registry() -> Registry {
    Registry::default()
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
            let paths = match &sandbox {
                SandboxDecision::Enabled(root) => Paths::sandbox(root),
                _ => Paths::real(app.path().app_config_dir()?, app.path().home_dir()?),
            };
            app.manage(AppState::load(paths, registry()));

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
            run_analysis
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
    fn the_app_starts_with_no_collectors_until_f03() {
        assert!(registry().collectors().is_empty());
    }
}
