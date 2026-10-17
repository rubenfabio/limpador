//! Desktop shell: wires the core library to the Tauri window. No business rules here (ADR-0003).

#[tauri::command]
fn core_version() -> &'static str {
    limpador_core::version()
}

pub fn run() {
    tauri::Builder::default()
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
