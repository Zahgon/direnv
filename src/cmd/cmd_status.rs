//! `direnv status`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::{format_string_bool_map, format_string_slice, Config};
use crate::cmd::env::Env;
use crate::cmd::rc::RC;
use crate::deps::gojson::{self, JsonValue};
use crate::deps::{gopath, gotime};
use crate::Result;

pub fn cmd_status() -> Cmd {
    Cmd {
        name: "status",
        desc: "Prints some debug status information".to_string(),
        args: &["[--json]"],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_status_action),
    }
}

fn cmd_status_action(_env: &Env, args: &[String], config: &Config) -> Result<()> {
    if args.len() > 1 && (args[1] == "-json" || args[1] == "--json") {
        let loaded_rc = config.loaded_rc();
        let found_rc = config.find_rc()?;

        let json_output = JsonValue::Object(vec![
            (
                "config".to_string(),
                JsonValue::sorted_object(vec![
                    (
                        "SelfPath".to_string(),
                        JsonValue::String(config.self_path.clone()),
                    ),
                    (
                        "ConfigDir".to_string(),
                        JsonValue::String(config.conf_dir.clone()),
                    ),
                ]),
            ),
            (
                "state".to_string(),
                JsonValue::sorted_object(vec![
                    ("loadedRC".to_string(), rc_json(loaded_rc.as_ref())),
                    ("foundRC".to_string(), rc_json(found_rc.as_ref())),
                ]),
            ),
        ]);
        println!("{}", gojson::marshal_indent(&json_output, "  "));
    } else {
        println!("direnv exec path {}", config.self_path);
        println!("DIRENV_CONFIG {}", config.conf_dir);

        println!("bash_path {}", config.bash_path);
        println!("disable_stdin {}", config.disable_stdin);
        println!(
            "warn_timeout {}",
            gotime::duration_string(config.warn_timeout)
        );
        println!(
            "whitelist.prefix {}",
            format_string_slice(&config.whitelist_prefix)
        );
        println!(
            "whitelist.exact {}",
            format_string_bool_map(&config.whitelist_exact)
        );

        let loaded_rc = config.loaded_rc();
        let found_rc = config.find_rc()?;

        match &loaded_rc {
            Some(rc) => format_rc("Loaded", rc),
            None => println!("No .envrc or .env loaded"),
        }

        match &found_rc {
            Some(rc) => format_rc("Found", rc),
            None => println!("No .envrc or .env found"),
        }
    }
    Ok(())
}

fn rc_json(rc: Option<&RC>) -> JsonValue {
    match rc {
        None => JsonValue::Null,
        Some(rc) => JsonValue::sorted_object(vec![
            ("path".to_string(), JsonValue::String(rc.path().to_string())),
            (
                "allowed".to_string(),
                JsonValue::Number(rc.allowed() as i32 as f64),
            ),
        ]),
    }
}

fn format_rc(desc: &str, rc: &RC) {
    let work_dir = gopath::dir(rc.path());

    println!("{desc} RC path {}", rc.path());
    for time in &rc.times().list {
        println!("{desc} watch: {}", time.formatted(&work_dir));
    }
    println!("{desc} RC allowed {}", rc.allowed());
    println!("{desc} RC allowPath {}", rc.allow_path());
}
