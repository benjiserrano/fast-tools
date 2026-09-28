// Sin consola en release; en debug se conserva para ver los logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fast_tools_lib::run()
}
