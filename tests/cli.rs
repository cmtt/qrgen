use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn qrgen() -> Command {
    let mut cmd = Command::cargo_bin("qrgen").unwrap();
    cmd.env_remove("NO_COLOR").env_remove("CLICOLOR_FORCE");
    cmd
}

fn stdout_of(args: &[&str], stdin: &[u8]) -> String {
    let out = qrgen()
        .args(args)
        .write_stdin(stdin)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(out).unwrap()
}

#[test]
fn invalid_margins_are_usage_errors() {
    for args in [
        &["-m", "x"][..],
        &["-m", "-3"],
        &["--margin=-3"],
        &["--svg", "--margin=-1"],
        &["-m", "99999999999"],
    ] {
        qrgen()
            .args(args)
            .write_stdin("foo")
            .assert()
            .code(2)
            .stderr(contains("panicked").not());
    }
}

#[test]
fn input_at_capacity_is_accepted() {
    qrgen().write_stdin("a".repeat(2953)).assert().success();
}

#[test]
fn input_over_capacity_is_an_error() {
    qrgen()
        .write_stdin("a".repeat(2954))
        .assert()
        .code(1)
        .stderr(contains("too long"))
        .stderr(contains("panicked").not());
}

#[test]
fn non_utf8_input_is_encoded_as_bytes() {
    qrgen().write_stdin(&b"\xff\xfe"[..]).assert().success();
}

#[test]
fn trailing_newline_is_trimmed_unless_raw() {
    let piped = stdout_of(&[], b"foobar\n");
    assert_eq!(piped, stdout_of(&["foobar"], b""));
    assert_eq!(piped, stdout_of(&[], b"foobar\r\n"));
    assert_ne!(piped, stdout_of(&["--raw"], b"foobar\n"));
}

#[test]
fn version_matches_cargo() {
    qrgen()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn small_halves_the_size() {
    let size = |s: &str| (s.lines().count(), s.lines().next().unwrap().chars().count());
    let (rows, cols) = size(&stdout_of(&["foobar"], b""));
    let (small_rows, small_cols) = size(&stdout_of(&["--small", "foobar"], b""));
    assert_eq!(cols, 2 * small_cols);
    assert_eq!(small_rows, rows.div_ceil(2));
}

#[test]
fn svg_conflicts_with_small() {
    qrgen().args(["--svg", "--small", "foo"]).assert().code(2);
}

#[test]
fn svg_is_well_formed() {
    let svg = stdout_of(&["--svg", "foobar"], b"");
    assert!(svg.starts_with("<?xml"));
    assert!(svg.trim_end().ends_with("</svg>"));
}

#[test]
fn piped_output_has_no_escape_codes() {
    assert!(!stdout_of(&["foobar"], b"").contains('\x1b'));
    assert!(!stdout_of(&["--small", "foobar"], b"").contains('\x1b'));
}

#[test]
fn color_always_uses_only_black_and_white() {
    for args in [
        &["--color", "always", "foobar"][..],
        &["--color", "always", "-s", "foobar"],
    ] {
        let out = stdout_of(args, b"");
        assert!(out.contains("\x1b[48;5;231m"));
        let mut codes: Vec<&str> = out
            .split('\x1b')
            .skip(1)
            .map(|s| &s[..=s.find('m').unwrap()])
            .collect();
        codes.sort();
        codes.dedup();
        for code in codes {
            assert!(
                ["[0m", "[38;5;16m", "[38;5;231m", "[48;5;16m", "[48;5;231m"].contains(&code),
                "unexpected escape code {:?}",
                code
            );
        }
    }
}

#[test]
fn color_environment_conventions() {
    let colored = |cmd: &mut Command| {
        let out = cmd
            .arg("foobar")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        String::from_utf8(out).unwrap().contains('\x1b')
    };
    assert!(colored(qrgen().env("CLICOLOR_FORCE", "1")));
    assert!(colored(
        qrgen().env("NO_COLOR", "1").args(["--color", "always"])
    ));
    assert!(!colored(
        qrgen()
            .env("CLICOLOR_FORCE", "1")
            .args(["--color", "never"])
    ));
}

#[test]
fn invert_changes_output_but_not_size() {
    let normal = stdout_of(&["foobar"], b"");
    let inverted = stdout_of(&["--invert", "foobar"], b"");
    assert_ne!(normal, inverted);
    assert_eq!(normal.lines().count(), inverted.lines().count());
}

#[test]
fn svg_conflicts_with_invert() {
    qrgen().args(["--svg", "--invert", "foo"]).assert().code(2);
}

#[test]
fn color_default_uses_no_glyphs() {
    let out = stdout_of(&["--color", "always", "foobar"], b"");
    assert!(!out.contains(['▀', '▄', '█']));
}

#[test]
fn dash_reads_stdin() {
    assert_eq!(stdout_of(&["-"], b"foobar\n"), stdout_of(&["foobar"], b""));
    assert_eq!(
        stdout_of(&["--small", "-"], b"x"),
        stdout_of(&["--small", "x"], b"")
    );
}
