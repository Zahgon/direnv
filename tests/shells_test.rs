//! Golden renderings of every shell back-end, captured from the original.
//!
//! The Go build's own suite covered only `BashEscape` and the systemd
//! sanitiser; the other ten escapers were exercised only through the shell
//! integration scripts. Every expectation below was produced by running
//! `direnv dump <shell>` against the Go binary at commit b00e451 with a
//! single-variable environment, so the table is evidence rather than a
//! restatement of the port.

use std::collections::HashMap;

use direnv::cmd::env::Env;
use direnv::cmd::shell::{detect_shell, Shell};

fn dump(shell: &str, key: &str, value: &str) -> String {
    let shell: &dyn Shell = detect_shell(shell).expect("known shell");
    let env: Env = HashMap::from([(key.to_string(), value.to_string())]).into();
    env.to_shell(shell).expect("render")
}

/// `direnv dump bash` for one variable at a time.
#[test]
fn bash_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "export V='';"),
        ("V", "plain", "export V=$'plain';"),
        ("V", "a b", "export V=$'a b';"),
        ("V", "it's", "export V=$'it\\'s';"),
        ("V", "say \"hi\"", "export V=$'say \"hi\"';"),
        ("V", "back\\slash", "export V=$'back\\\\slash';"),
        ("V", "a\tb\nc", "export V=$'a\\tb\\nc';"),
        ("V", "$HOME `x`", "export V=$'$HOME `x`';"),
        ("V", "é♥", "export V=$'\\xc3\\xa9\\xe2\\x99\\xa5';"),
        ("V", "/a/b:/c/d", "export V=$'/a/b:/c/d';"),
        ("V", "a<b>&c", "export V=$'a<b>&c';"),
        ("V", "\u{1}\u{7f}", "export V=$'\\x01\\x7f';"),
        ("V", "val", "export V=$'val';"),
        ("PATH", "val", "export PATH=$'val';"),
        ("a-b", "val", "export $'a-b'=$'val';"),
        ("x*y", "val", "export $'x*y'=$'val';"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("bash", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump zsh` for one variable at a time.
#[test]
fn zsh_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "export V='';"),
        ("V", "plain", "export V=$'plain';"),
        ("V", "a b", "export V=$'a b';"),
        ("V", "it's", "export V=$'it\\'s';"),
        ("V", "say \"hi\"", "export V=$'say \"hi\"';"),
        ("V", "back\\slash", "export V=$'back\\\\slash';"),
        ("V", "a\tb\nc", "export V=$'a\\tb\\nc';"),
        ("V", "$HOME `x`", "export V=$'$HOME `x`';"),
        ("V", "é♥", "export V=$'\\xc3\\xa9\\xe2\\x99\\xa5';"),
        ("V", "/a/b:/c/d", "export V=$'/a/b:/c/d';"),
        ("V", "a<b>&c", "export V=$'a<b>&c';"),
        ("V", "\u{1}\u{7f}", "export V=$'\\x01\\x7f';"),
        ("V", "val", "export V=$'val';"),
        ("PATH", "val", "export PATH=$'val';"),
        ("a-b", "val", "export $'a-b'=$'val';"),
        ("x*y", "val", "export $'x*y'=$'val';"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("zsh", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump fish` for one variable at a time.
#[test]
fn fish_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "set -x -g 'V' '';"),
        ("V", "plain", "set -x -g 'V' 'plain';"),
        ("V", "a b", "set -x -g 'V' 'a b';"),
        ("V", "it's", "set -x -g 'V' 'it\\'s';"),
        ("V", "say \"hi\"", "set -x -g 'V' 'say \"hi\"';"),
        ("V", "back\\slash", "set -x -g 'V' 'back\\\\slash';"),
        ("V", "a\tb\nc", "set -x -g 'V' 'a'\\t'b'\\n'c';"),
        ("V", "$HOME `x`", "set -x -g 'V' '$HOME `x`';"),
        (
            "V",
            "é♥",
            "set -x -g 'V' ''\\Xc3''\\Xa9''\\Xe2''\\X99''\\Xa5'';",
        ),
        ("V", "/a/b:/c/d", "set -x -g 'V' '/a/b:/c/d';"),
        ("V", "a<b>&c", "set -x -g 'V' 'a<b>&c';"),
        ("V", "\u{1}\u{7f}", "set -x -g 'V' ''\\X01''\\X7f'';"),
        ("V", "val", "set -x -g 'V' 'val';"),
        ("PATH", "val", "set -x -g PATH 'val';"),
        ("a-b", "val", "set -x -g 'a-b' 'val';"),
        ("x*y", "val", "set -x -g 'x*y' 'val';"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("fish", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump tcsh` for one variable at a time.
#[test]
fn tcsh_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "setenv V '' ;"),
        ("V", "plain", "setenv V plain ;"),
        ("V", "a b", "setenv V a\\ b ;"),
        ("V", "it's", "setenv V it\\'s ;"),
        ("V", "say \"hi\"", "setenv V say\\ \"\"\"hi\"\"\" ;"),
        ("V", "back\\slash", "setenv V back\\\\slash ;"),
        ("V", "a\tb\nc", "setenv V a\\tb\\nc ;"),
        ("V", "$HOME `x`", "setenv V \"$\"HOME\\ `x` ;"),
        ("V", "é♥", "setenv V \\xc3\\xa9\\xe2\\x99\\xa5 ;"),
        ("V", "/a/b:/c/d", "setenv V /a/b\":\"/c/d ;"),
        ("V", "a<b>&c", "setenv V a\"<\"b\">\"\"&\"c ;"),
        ("V", "\u{1}\u{7f}", "setenv V \\x01\\x7f ;"),
        ("V", "val", "setenv V val ;"),
        ("PATH", "val", "set path = ( val );"),
        ("a-b", "val", "setenv a-b val ;"),
        ("x*y", "val", "setenv x\"*\"y val ;"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("tcsh", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump vim` for one variable at a time.
#[test]
fn vim_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "call setenv('V','')\n"),
        ("V", "plain", "call setenv('V','plain')\n"),
        ("V", "a b", "call setenv('V','a b')\n"),
        ("V", "it's", "call setenv('V','it''s')\n"),
        ("V", "say \"hi\"", "call setenv('V','say \"hi\"')\n"),
        ("V", "back\\slash", "call setenv('V','back\\slash')\n"),
        ("V", "a\tb\nc", "call setenv('V','a\tb\\nc')\n"),
        ("V", "$HOME `x`", "call setenv('V','$HOME `x`')\n"),
        ("V", "é♥", "call setenv('V','é♥')\n"),
        ("V", "/a/b:/c/d", "call setenv('V','/a/b:/c/d')\n"),
        ("V", "a<b>&c", "call setenv('V','a<b>&c')\n"),
        ("V", "\u{1}\u{7f}", "call setenv('V','\u{1}\u{7f}')\n"),
        ("V", "val", "call setenv('V','val')\n"),
        ("PATH", "val", "call setenv('PATH','val')\n"),
        ("a-b", "val", "call setenv('a-b','val')\n"),
        ("x*y", "val", "call setenv('x*y','val')\n"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("vim", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump pwsh` for one variable at a time.
#[test]
fn pwsh_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "${env:V}='';"),
        ("V", "plain", "${env:V}='plain';"),
        ("V", "a b", "${env:V}='a b';"),
        ("V", "it's", "${env:V}='it''s';"),
        ("V", "say \"hi\"", "${env:V}='say \"hi\"';"),
        ("V", "back\\slash", "${env:V}='back\\slash';"),
        ("V", "a\tb\nc", "${env:V}='a\tb\nc';"),
        ("V", "$HOME `x`", "${env:V}='$HOME `x`';"),
        ("V", "é♥", "${env:V}='é♥';"),
        ("V", "/a/b:/c/d", "${env:V}='/a/b:/c/d';"),
        ("V", "a<b>&c", "${env:V}='a<b>&c';"),
        ("V", "\u{1}\u{7f}", "${env:V}='\u{1}\u{7f}';"),
        ("V", "val", "${env:V}='val';"),
        ("PATH", "val", "${env:PATH}='val';"),
        ("a-b", "val", "${env:a-b}='val';"),
        ("x*y", "val", "${env:x\\x2ay}='val';"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("pwsh", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump systemd` for one variable at a time.
#[test]
fn systemd_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "V=\n"),
        ("V", "plain", "V=plain\n"),
        ("V", "a b", "V=a b\n"),
        ("V", "it's", "V=\"it's\"\n"),
        ("V", "say \"hi\"", "V=\"say \\\"hi\\\"\"\n"),
        ("V", "back\\slash", "V=\"back\\slash\"\n"),
        ("V", "a\tb\nc", "V=\"a\tb\nc\"\n"),
        ("V", "$HOME `x`", "V=$HOME `x`\n"),
        ("V", "é♥", "V=é♥\n"),
        ("V", "/a/b:/c/d", "V=/a/b:/c/d\n"),
        ("V", "a<b>&c", "V=a<b>&c\n"),
        ("V", "\u{1}\u{7f}", "V=\u{1}\u{7f}\n"),
        ("V", "val", "V=val\n"),
        ("PATH", "val", "PATH=val\n"),
        ("a-b", "val", "a-b=val\n"),
        ("x*y", "val", "x*y=val\n"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("systemd", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump json` for one variable at a time.
#[test]
fn json_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "{\n  \"V\": \"\"\n}"),
        ("V", "plain", "{\n  \"V\": \"plain\"\n}"),
        ("V", "a b", "{\n  \"V\": \"a b\"\n}"),
        ("V", "it's", "{\n  \"V\": \"it's\"\n}"),
        ("V", "say \"hi\"", "{\n  \"V\": \"say \\\"hi\\\"\"\n}"),
        ("V", "back\\slash", "{\n  \"V\": \"back\\\\slash\"\n}"),
        ("V", "a\tb\nc", "{\n  \"V\": \"a\\tb\\nc\"\n}"),
        ("V", "$HOME `x`", "{\n  \"V\": \"$HOME `x`\"\n}"),
        ("V", "é♥", "{\n  \"V\": \"é♥\"\n}"),
        ("V", "/a/b:/c/d", "{\n  \"V\": \"/a/b:/c/d\"\n}"),
        ("V", "a<b>&c", "{\n  \"V\": \"a\\u003cb\\u003e\\u0026c\"\n}"),
        ("V", "\u{1}\u{7f}", "{\n  \"V\": \"\\u0001\u{7f}\"\n}"),
        ("V", "val", "{\n  \"V\": \"val\"\n}"),
        ("PATH", "val", "{\n  \"PATH\": \"val\"\n}"),
        ("a-b", "val", "{\n  \"a-b\": \"val\"\n}"),
        ("x*y", "val", "{\n  \"x*y\": \"val\"\n}"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("json", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump elvish` for one variable at a time.
#[test]
fn elvish_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "{\"V\":\"\"}\n"),
        ("V", "plain", "{\"V\":\"plain\"}\n"),
        ("V", "a b", "{\"V\":\"a b\"}\n"),
        ("V", "it's", "{\"V\":\"it's\"}\n"),
        ("V", "say \"hi\"", "{\"V\":\"say \\\"hi\\\"\"}\n"),
        ("V", "back\\slash", "{\"V\":\"back\\\\slash\"}\n"),
        ("V", "a\tb\nc", "{\"V\":\"a\\tb\\nc\"}\n"),
        ("V", "$HOME `x`", "{\"V\":\"$HOME `x`\"}\n"),
        ("V", "é♥", "{\"V\":\"é♥\"}\n"),
        ("V", "/a/b:/c/d", "{\"V\":\"/a/b:/c/d\"}\n"),
        ("V", "a<b>&c", "{\"V\":\"a\\u003cb\\u003e\\u0026c\"}\n"),
        ("V", "\u{1}\u{7f}", "{\"V\":\"\\u0001\u{7f}\"}\n"),
        ("V", "val", "{\"V\":\"val\"}\n"),
        ("PATH", "val", "{\"PATH\":\"val\"}\n"),
        ("a-b", "val", "{\"a-b\":\"val\"}\n"),
        ("x*y", "val", "{\"x*y\":\"val\"}\n"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("elvish", key, value),
            "key={key:?} value={value:?}"
        );
    }
}

/// `direnv dump murex` for one variable at a time.
#[test]
fn murex_rendering_matches_the_original() {
    let cases: &[(&str, &str, &str)] = &[
        ("V", "", "{\"V\":\"\"}\n"),
        ("V", "plain", "{\"V\":\"plain\"}\n"),
        ("V", "a b", "{\"V\":\"a b\"}\n"),
        ("V", "it's", "{\"V\":\"it's\"}\n"),
        ("V", "say \"hi\"", "{\"V\":\"say \\\"hi\\\"\"}\n"),
        ("V", "back\\slash", "{\"V\":\"back\\\\slash\"}\n"),
        ("V", "a\tb\nc", "{\"V\":\"a\\tb\\nc\"}\n"),
        ("V", "$HOME `x`", "{\"V\":\"$HOME `x`\"}\n"),
        ("V", "é♥", "{\"V\":\"é♥\"}\n"),
        ("V", "/a/b:/c/d", "{\"V\":\"/a/b:/c/d\"}\n"),
        ("V", "a<b>&c", "{\"V\":\"a\\u003cb\\u003e\\u0026c\"}\n"),
        ("V", "\u{1}\u{7f}", "{\"V\":\"\\u0001\u{7f}\"}\n"),
        ("V", "val", "{\"V\":\"val\"}\n"),
        ("PATH", "val", "{\"PATH\":\"val\"}\n"),
        ("a-b", "val", "{\"a-b\":\"val\"}\n"),
        ("x*y", "val", "{\"x*y\":\"val\"}\n"),
    ];
    for (key, value, expected) in cases {
        assert_eq!(
            *expected,
            dump("murex", key, value),
            "key={key:?} value={value:?}"
        );
    }
}
