//! The bash stdlib that `.envrc` files are evaluated against.

use crate::cmd::config::Config;

/// Returns the stdlib.sh, with references to direnv replaced.
pub fn get_stdlib(config: &Config) -> String {
    crate::cmd::stdlib_source().replacen("$(command -v direnv)", &config.self_path, 1)
}
