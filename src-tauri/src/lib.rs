//! Desktop shell: wires the core library to the Tauri window. No business rules here (ADR-0003).

pub mod sandbox;

use sandbox::{SANDBOX_ENV, SandboxDecision};

#[tauri::command]
fn core_version() -> &'static str {
    limpador_core::version()
}

// motivo: só abre a janela e liga as peças; as decisões estão em `sandbox` (com testes)
// e a abertura real é coberta pelo e2e do app (T-00.04).
#[cfg_attr(test, mutants::skip)]
pub fn run() {
    let sandbox = sandbox::decide(std::env::var_os(SANDBOX_ENV));
    if let Some(message) = sandbox::refusal_message(&sandbox) {
        eprintln!("{message}");
        std::process::exit(2);
    }

    tauri::Builder::default()
        // The main window is created here (not by tauri.conf.json) so that a sandboxed
        // run can keep the webview profile inside the fake machine.
        .setup(move |app| {
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
        .invoke_handler(tauri::generate_handler![core_version])
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
}
