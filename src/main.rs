mod app;
mod cli;
mod db;
mod error;
mod models;
mod utils;

use app::App;

fn main() {
    let json_output = std::env::args().any(|arg| arg == "--json");
    if let Err(e) = App::run() {
        if json_output {
            let payload = serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            });
            println!("{}", payload);
        } else {
            eprintln!("blackboard: {}", e);
        }
        std::process::exit(1);
    }
}
