// Sem console em release: o app vive na bandeja. Em debug o console fica, para
// os logs do `tauri dev`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    falcao_token_router_lib::run();
}
