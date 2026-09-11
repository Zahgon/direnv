//! Ported from `internal/cmd/shell_systemd_test.go`.

use std::collections::HashMap;

use direnv::cmd::env::Env;
use direnv::cmd::shell::SYSTEMD;
use direnv::cmd::shell_systemd::cut_encapsulated;

/// `TestCutEncapsulated_ok`
#[test]
fn test_cut_encapsulated_ok() {
    let input = "\"encapsulated string with \n line return\"";
    let (cut_input, is_encapsulated) = cut_encapsulated(input, "\"");
    assert!(is_encapsulated, "Test TestCutEncapsulated_ok failing.");
    assert_eq!("encapsulated string with \n line return", cut_input);
}

/// `TestExport_ok`
#[test]
fn test_export_ok() {
    let env: Env = HashMap::from([
        ("Key".to_string(), " just a Value".to_string()),
        (
            "Ex1".to_string(),
            r"'single quotes ' works like that'".to_string(),
        ),
        (
            "Ex2".to_string(),
            r"however, you can't use quotes inline".to_string(),
        ),
        (
            "Ex3".to_string(),
            r#"double quotes " are doing the "same" way"#.to_string(),
        ),
        (
            "Ex4".to_string(),
            r"quotes allows escapes: \n \x".to_string(),
        ),
        (
            "Ex5".to_string(),
            r#"quotes are doing trick for chars: ", \, $"#.to_string(),
        ),
        (
            "Ex6".to_string(),
            r"naked values allows escapes: \a, \b, \c".to_string(),
        ),
        (
            "Ex7".to_string(),
            r"and even \$, and even at the beginning".to_string(),
        ),
        ("Ex8".to_string(), r"\x all the rest".to_string()),
        (
            "Ex9".to_string(),
            r"in naked values backslash \allows splitting values".to_string(),
        ),
        ("Ex10".to_string(), r"quotes\nallow multi lines".to_string()),
        (
            "Ex11".to_string(),
            r"'with single quotes around it and '' single quotes in it '''".to_string(),
        ),
        (
            "Ex12".to_string(),
            r#""with quotes around it and quotes "" in "" it""#.to_string(),
        ),
    ])
    .into();

    let actual_output = env.to_shell(&SYSTEMD).expect("ToShell() failed");

    let expected_output_map: Vec<(&str, &str)> = vec![
        ("Key", " just a Value"),
        ("Ex1", "'single quotes \\' works like that'"),
        ("Ex2", "\"however, you can't use quotes inline\""),
        (
            "Ex3",
            "\"double quotes \\\" are doing the \\\"same\\\" way\"",
        ),
        ("Ex4", r#""quotes allows escapes: \n \x""#),
        ("Ex5", "\"quotes are doing trick for chars: \\\", \\, $\""),
        ("Ex6", "\"naked values allows escapes: \\a, \\b, \\c\""),
        ("Ex7", "\"and even \\$, and even at the beginning\""),
        ("Ex8", "\"\\x all the rest\""),
        (
            "Ex9",
            "\"in naked values backslash \\allows splitting values\"",
        ),
        ("Ex10", r#""quotes\nallow multi lines""#),
        (
            "Ex11",
            r"'with single quotes around it and \'\' single quotes in it \'\''",
        ),
        (
            "Ex12",
            r#""with quotes around it and quotes \"\" in \"\" it""#,
        ),
    ];

    for (key, expected_value) in expected_output_map {
        let needle = format!("{key}=");
        let after_prefix = actual_output
            .split_once(&needle)
            .map(|(_, rest)| rest)
            .expect("Test for systemd shell failed, couldn't not found expected key=value");
        let actual_value = after_prefix
            .split_once('\n')
            .map(|(head, _)| head)
            .expect("Test for systemd shell failed, couldn't not found expected key=value");
        assert_eq!(expected_value, actual_value);
    }
}
