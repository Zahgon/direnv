//! Ported from `pkg/dotenv/parse_test.go`.

use std::collections::HashMap;

use direnv::dotenv;

fn should_not_have_empty_key(env: &HashMap<String, String>) {
    assert!(!env.contains_key(""), "should not have empty key");
}

fn env_should_contain(env: &HashMap<String, String>, key: &str, value: &str) {
    let actual = env.get(key).cloned().unwrap_or_default();
    assert_eq!(value, actual, "{key}: {actual}, expected {value}");
}

fn get(env: &HashMap<String, String>, key: &str) -> String {
    env.get(key).cloned().unwrap_or_default()
}

// See the reference implementation:
//   https://github.com/bkeepers/dotenv/blob/master/lib/dotenv/environment.rb
// TODO: support shell variable expansions

const TEST_EXPORTED_ENV: &str = r#"export OPTION_A=2
export OPTION_B='\n' # foo
#export OPTION_C=3
export OPTION_D=
export OPTION_E="foo"
"#;

/// `TestDotEnvExported`
#[test]
fn test_dot_env_exported() {
    let env = dotenv::must_parse(TEST_EXPORTED_ENV);
    should_not_have_empty_key(&env);

    assert_eq!("2", get(&env, "OPTION_A"), "OPTION_A");
    assert_eq!("\\n", get(&env, "OPTION_B"), "OPTION_B");
    assert_eq!("", get(&env, "OPTION_C"), "OPTION_C");
    assert_eq!(Some(&String::new()), env.get("OPTION_D"), "OPTION_D");
    assert_eq!("foo", get(&env, "OPTION_E"), "OPTION_E");
}

const TEST_PLAIN_ENV: &str = r#"OPTION_A=1
OPTION_B=2
OPTION_C= 3
OPTION_D =4
OPTION_E = 5
OPTION_F=
OPTION_G =
SMTP_ADDRESS=smtp    # This is a comment
"#;

/// `TestDotEnvPlain`
#[test]
fn test_dot_env_plain() {
    let env = dotenv::must_parse(TEST_PLAIN_ENV);
    should_not_have_empty_key(&env);

    assert_eq!("1", get(&env, "OPTION_A"), "OPTION_A");
    assert_eq!("2", get(&env, "OPTION_B"), "OPTION_B");
    assert_eq!("3", get(&env, "OPTION_C"), "OPTION_C");
    assert_eq!("4", get(&env, "OPTION_D"), "OPTION_D");
    assert_eq!("5", get(&env, "OPTION_E"), "OPTION_E");
    assert_eq!(Some(&String::new()), env.get("OPTION_F"), "OPTION_F");
    assert_eq!(Some(&String::new()), env.get("OPTION_G"), "OPTION_G");
    assert_eq!("smtp", get(&env, "SMTP_ADDRESS"), "SMTP_ADDRESS");
}

const TEST_SOLO_EMPTY_ENV: &str = "SOME_VAR=";

/// `TestSoloEmpty`
#[test]
fn test_solo_empty() {
    let env = dotenv::must_parse(TEST_SOLO_EMPTY_ENV);
    should_not_have_empty_key(&env);

    let v = env.get("SOME_VAR");
    assert!(v.is_some(), "SOME_VAR missing");
    assert_eq!("", v.unwrap(), "SOME_VAR should be empty");
}

const TEST_QUOTED_ENV: &str = r#"OPTION_A='1'
OPTION_B='2'
OPTION_C=''
OPTION_D='\n'
OPTION_E="1"
OPTION_F="2"
OPTION_G=""
OPTION_H="\n"
#OPTION_I="3"
"#;

/// `TestDotEnvQuoted`
#[test]
fn test_dot_env_quoted() {
    let env = dotenv::must_parse(TEST_QUOTED_ENV);
    should_not_have_empty_key(&env);

    assert_eq!("1", get(&env, "OPTION_A"), "OPTION_A");
    assert_eq!("2", get(&env, "OPTION_B"), "OPTION_B");
    assert_eq!("", get(&env, "OPTION_C"), "OPTION_C");
    assert_eq!("\\n", get(&env, "OPTION_D"), "OPTION_D");
    assert_eq!("1", get(&env, "OPTION_E"), "OPTION_E");
    assert_eq!("2", get(&env, "OPTION_F"), "OPTION_F");
    assert_eq!("", get(&env, "OPTION_G"), "OPTION_G");
    assert_eq!("\n", get(&env, "OPTION_H"), "OPTION_H");
    assert_eq!("", get(&env, "OPTION_I"), "OPTION_I");
}

const TEST_YAML_ENV: &str =
    "OPTION_A: 1\nOPTION_B: '2'\nOPTION_C: ''\nOPTION_D: '\\n'\n#OPTION_E: '333'\nOPTION_F: \n";

/// `TestDotEnvYAML`
#[test]
fn test_dot_env_yaml() {
    let env = dotenv::must_parse(TEST_YAML_ENV);
    should_not_have_empty_key(&env);

    assert_eq!("1", get(&env, "OPTION_A"), "OPTION_A");
    assert_eq!("2", get(&env, "OPTION_B"), "OPTION_B");
    assert_eq!("", get(&env, "OPTION_C"), "OPTION_C");
    assert_eq!("\\n", get(&env, "OPTION_D"), "OPTION_D");
    assert_eq!("", get(&env, "OPTION_E"), "OPTION_E");
    assert_eq!(Some(&String::new()), env.get("OPTION_F"), "OPTION_F");
}

/// `TestFailingMustParse`
#[test]
#[should_panic]
fn test_failing_must_parse() {
    dotenv::must_parse("...");
}

const TEST_COMMENT_OVERRIDE_ENV: &str = r#"
VARIABLE=value
#VARIABLE=disabled_value
"#;

/// `TestCommentOverride`
#[test]
fn test_comment_override() {
    let env = dotenv::must_parse(TEST_COMMENT_OVERRIDE_ENV);
    should_not_have_empty_key(&env);

    assert_eq!(
        "value",
        get(&env, "VARIABLE"),
        "VARIABLE should == value, not {}",
        get(&env, "VARIABLE")
    );
}

const TEST_VARIABLE_EXPANSION_ENV: &str = r#"
OPTION_A=$FOO
OPTION_B="$FOO"
OPTION_C=${FOO}
OPTION_D="${FOO}"
OPTION_E='$FOO'
OPTION_F=$FOO/bar
OPTION_G="$FOO/bar"
OPTION_H=${FOO}/bar
OPTION_I="${FOO}/bar"
OPTION_J='$FOO/bar'
OPTION_K=$BAR
OPTION_L="$BAR"
OPTION_M=${BAR}
OPTION_N="${BAR}"
OPTION_O='$BAR'
OPTION_P=$BAR/baz
OPTION_Q="$BAR/baz"
OPTION_R=${BAR}/baz
OPTION_S="${BAR}/baz"
OPTION_T='$BAR/baz'
OPTION_U="$OPTION_A/bar"
OPTION_V=$OPTION_A/bar
OPTION_W="$OPTION_A/bar"
OPTION_X=${OPTION_A}/bar
OPTION_Y="${OPTION_A}/bar"
OPTION_Z='$OPTION_A/bar'
OPTION_A1="$OPTION_A/bar/${OPTION_H}/$FOO"
"#;

/// `TestVariableExpansion`
#[test]
fn test_variable_expansion() {
    std::env::set_var("FOO", "foo");

    let env = dotenv::must_parse(TEST_VARIABLE_EXPANSION_ENV);
    should_not_have_empty_key(&env);

    env_should_contain(&env, "OPTION_A", "foo");
    env_should_contain(&env, "OPTION_B", "foo");
    env_should_contain(&env, "OPTION_C", "foo");
    env_should_contain(&env, "OPTION_D", "foo");
    env_should_contain(&env, "OPTION_E", "$FOO");
    env_should_contain(&env, "OPTION_F", "foo/bar");
    env_should_contain(&env, "OPTION_G", "foo/bar");
    env_should_contain(&env, "OPTION_H", "foo/bar");
    env_should_contain(&env, "OPTION_I", "foo/bar");
    env_should_contain(&env, "OPTION_J", "$FOO/bar");
    env_should_contain(&env, "OPTION_K", "");
    env_should_contain(&env, "OPTION_L", "");
    env_should_contain(&env, "OPTION_M", "");
    env_should_contain(&env, "OPTION_N", "");
    env_should_contain(&env, "OPTION_O", "$BAR");
    env_should_contain(&env, "OPTION_P", "/baz");
    env_should_contain(&env, "OPTION_Q", "/baz");
    env_should_contain(&env, "OPTION_R", "/baz");
    env_should_contain(&env, "OPTION_S", "/baz");
    env_should_contain(&env, "OPTION_T", "$BAR/baz");
    env_should_contain(&env, "OPTION_U", "foo/bar");
    env_should_contain(&env, "OPTION_V", "foo/bar");
    env_should_contain(&env, "OPTION_W", "foo/bar");
    env_should_contain(&env, "OPTION_X", "foo/bar");
    env_should_contain(&env, "OPTION_Y", "foo/bar");
    env_should_contain(&env, "OPTION_Z", "$OPTION_A/bar");
    env_should_contain(&env, "OPTION_A1", "foo/bar/foo/bar/foo");
}

const TEST_VARIABLE_EXPANSION_WITH_DEFAULTS_ENV: &str = r#"
OPTION_A="${FOO:-}"
OPTION_B="${FOO:-default}"
OPTION_C='${FOO:-default}'
OPTION_D="${FOO:-default}/bar"
OPTION_E='${FOO:-default}/bar'
OPTION_F="$FOO:-default"
OPTION_G="$BAR:-default"
OPTION_H="${BAR:-}"
OPTION_I="${BAR:-default}"
OPTION_J='${BAR:-default}'
OPTION_K="${BAR:-default}/bar"
OPTION_L='${BAR:-default}/bar'
OPTION_M="${OPTION_A:-}"
OPTION_N="${OPTION_A:-default}"
OPTION_O='${OPTION_A:-default}'
OPTION_P="${OPTION_A:-default}/bar"
OPTION_Q='${OPTION_A:-default}/bar'
OPTION_R="${:-}"
OPTION_S="${BAR:-:-}"
"#;

/// `TestVariableExpansionWithDefaults`
#[test]
fn test_variable_expansion_with_defaults() {
    std::env::set_var("FOO", "foo");

    let env = dotenv::must_parse(TEST_VARIABLE_EXPANSION_WITH_DEFAULTS_ENV);
    should_not_have_empty_key(&env);

    env_should_contain(&env, "OPTION_A", "foo");
    env_should_contain(&env, "OPTION_B", "foo");
    env_should_contain(&env, "OPTION_C", "${FOO:-default}");
    env_should_contain(&env, "OPTION_D", "foo/bar");
    env_should_contain(&env, "OPTION_E", "${FOO:-default}/bar");
    env_should_contain(&env, "OPTION_F", "foo:-default");
    env_should_contain(&env, "OPTION_G", ":-default");
    env_should_contain(&env, "OPTION_H", "");
    env_should_contain(&env, "OPTION_I", "default");
    env_should_contain(&env, "OPTION_J", "${BAR:-default}");
    env_should_contain(&env, "OPTION_K", "default/bar");
    env_should_contain(&env, "OPTION_L", "${BAR:-default}/bar");
    env_should_contain(&env, "OPTION_M", "foo");
    env_should_contain(&env, "OPTION_N", "foo");
    env_should_contain(&env, "OPTION_O", "${OPTION_A:-default}");
    env_should_contain(&env, "OPTION_P", "foo/bar");
    env_should_contain(&env, "OPTION_Q", "${OPTION_A:-default}/bar");
    // this is actually invalid in bash, but what to do here?
    env_should_contain(&env, "OPTION_R", "");
    env_should_contain(&env, "OPTION_S", ":-");
}

const TEST_MULTILINE_ENV: &str = "MULTILINE=\"line1\nline2\nline3\"\nSINGLE=one";

/// `TestDotEnvMultiline`
#[test]
fn test_dot_env_multiline() {
    let env = dotenv::must_parse(TEST_MULTILINE_ENV);
    should_not_have_empty_key(&env);

    env_should_contain(&env, "MULTILINE", "line1\nline2\nline3");
    env_should_contain(&env, "SINGLE", "one");
}

/// `TestDotEnvUnclosedQuote`
#[test]
#[should_panic]
fn test_dot_env_unclosed_quote() {
    dotenv::must_parse("FOO=\"line1\nline2");
}

const TEST_MIXED_MULTILINE_ENV: &str = "A=1\nB=\"foo\nbar\"\nC=3";

/// `TestDotEnvMixedMultiline`
#[test]
fn test_dot_env_mixed_multiline() {
    let env = dotenv::must_parse(TEST_MIXED_MULTILINE_ENV);
    should_not_have_empty_key(&env);

    env_should_contain(&env, "A", "1");
    env_should_contain(&env, "B", "foo\nbar");
    env_should_contain(&env, "C", "3");
}

const TEST_NESTED_JSON_ENV: &str = r#"CONFIG='{
  "key1": "value1",
  "key2": "value2",
  "nested": {
    "nested_key_1": "nested_value_1"
  }
}'
OTHER=value"#;

/// `TestDotEnvNestedJSON`
#[test]
fn test_dot_env_nested_json() {
    let env = dotenv::must_parse(TEST_NESTED_JSON_ENV);
    should_not_have_empty_key(&env);

    let expected_json = "{\n  \"key1\": \"value1\",\n  \"key2\": \"value2\",\n  \"nested\": {\n    \"nested_key_1\": \"nested_value_1\"\n  }\n}";
    env_should_contain(&env, "CONFIG", expected_json);
    env_should_contain(&env, "OTHER", "value");
}
