//! The direnv command-line tool.

use std::process::ExitCode;

use direnv::cmd;
use direnv::cmd::env::Env;

/// Configured at compile time; the build passes `BASH_PATH` through when it is
/// set, matching the original's `-ldflags -X main.bashPath=...`.
const BASH_PATH: &str = match option_env!("BASH_PATH") {
    Some(path) => path,
    None => "",
};

const STDLIB: &str = include_str!("../stdlib.sh");

const VERSION: &str = include_str!("../version.txt");

fn main() -> ExitCode {
    let env = Env::from_process();
    let args: Vec<String> = std::env::args().collect();

    match cmd::main(&env, &args, BASH_PATH, STDLIB, VERSION.trim()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
