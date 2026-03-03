#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --conky-only: just generate conky.conf and start conky, no UI
    #[cfg(target_os = "linux")]
    if args.iter().any(|a| a == "--conky-only") {
        an8nymous_stats_lib::conky_only();
        return;
    }

    an8nymous_stats_lib::run();
}
