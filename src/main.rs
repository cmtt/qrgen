mod render;

use clap::{CommandFactory, Parser, ValueEnum};
use qrcodegen::{QrCode, QrCodeEcc, QrSegment, Version};
use std::io::{self, BufWriter, IsTerminal, Read, Write};
use std::process::ExitCode;

/// Renders text as QR codes on Unicode terminals or as SVG.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Text to encode. Reads stdin if omitted or "-".
    text: Option<String>,

    /// Half the height using ▀▄█ glyphs (needs a font that draws them exactly).
    #[arg(short, long)]
    small: bool,

    /// Quiet-zone width in modules.
    #[arg(short, long, default_value_t = 4, value_name = "MODULES")]
    margin: u8,

    /// Write an SVG image instead.
    #[arg(long, conflicts_with = "small")]
    svg: bool,

    /// Keep a trailing newline read from stdin.
    #[arg(long)]
    raw: bool,

    /// Paint black on white regardless of theme (auto: terminals only, honours NO_COLOR).
    #[arg(long, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    color: ColorWhen,

    /// Swap dark and light modules (for dark themes with --color never).
    #[arg(short, long, conflicts_with = "svg")]
    invert: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, ValueEnum)]
enum ColorWhen {
    // When writing to a terminal, unless NO_COLOR is set or TERM=dumb.
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Waiting silently on a terminal looks like a hang; "-" opts back in.
    if cli.text.is_none() && io::stdin().is_terminal() {
        let _ = Cli::command().write_help(&mut io::stderr());
        return ExitCode::from(2);
    }

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qrgen: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let input = read_input(cli.text.as_deref(), cli.raw, &mut io::stdin().lock())
        .map_err(|e| format!("failed to read input: {}", e))?;
    let qr = encode(&input)?;

    let opts = render::TermOpts {
        margin: cli.margin,
        invert: cli.invert,
        color: use_color(cli.color, io::stdout().is_terminal(), |name| {
            std::env::var(name).ok()
        }),
    };

    let mut out = BufWriter::new(io::stdout().lock());
    let result = if cli.svg {
        render::svg(&qr, cli.margin, &mut out)
    } else if cli.small {
        render::half_blocks(&qr, &opts, &mut out)
    } else {
        render::full_blocks(&qr, &opts, &mut out)
    };
    match result.and_then(|()| out.flush()) {
        Err(e) if e.kind() != io::ErrorKind::BrokenPipe => {
            Err(format!("failed to write output: {}", e))
        }
        _ => Ok(()),
    }
}

/// Returns the text argument, or `stdin` if it's absent or "-", minus one
/// trailing newline from stdin unless `raw` is set.
fn read_input(text: Option<&str>, raw: bool, stdin: &mut impl Read) -> io::Result<Vec<u8>> {
    if let Some(text) = text.filter(|&t| t != "-") {
        return Ok(text.as_bytes().to_vec());
    }
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;
    if !raw && buffer.last() == Some(&b'\n') {
        buffer.pop();
        if buffer.last() == Some(&b'\r') {
            buffer.pop();
        }
    }
    Ok(buffer)
}

fn encode(input: &[u8]) -> Result<QrCode, String> {
    let segs = match std::str::from_utf8(input) {
        Ok(text) => QrSegment::make_segments(text),
        Err(_) => vec![QrSegment::make_bytes(input)],
    };
    // Mask None picks the lowest-penalty mask; boost_ecl raises the error
    // correction level as far as it goes without growing the symbol.
    QrCode::encode_segments_advanced(
        &segs,
        QrCodeEcc::Low,
        Version::MIN,
        Version::MAX,
        None,
        true,
    )
    .map_err(|e| {
        format!(
            "input is {} bytes, too long for a QR code (max 2953 bytes in byte mode): {}",
            input.len(),
            e
        )
    })
}

/// Decides whether to paint with colours, following the NO_COLOR and
/// CLICOLOR_FORCE conventions in auto mode.
fn use_color(when: ColorWhen, stdout_is_tty: bool, env: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| env(name).is_some_and(|v| !v.is_empty());
    match when {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => {
            if set("NO_COLOR") {
                false
            } else if set("CLICOLOR_FORCE") && env("CLICOLOR_FORCE").as_deref() != Some("0") {
                true
            } else {
                stdout_is_tty && env("TERM").as_deref() != Some("dumb")
            }
        }
    }
}

#[cfg(test)]
mod testutil {
    /// Deterministic pseudo-random bytes (xorshift64) for reproducible tests.
    pub fn bytes(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn color_with(when: ColorWhen, tty: bool, vars: &[(&str, &str)]) -> bool {
        use_color(when, tty, |name| {
            vars.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn auto_colors_terminals_only() {
        let term = [("TERM", "xterm-256color")];
        assert!(color_with(ColorWhen::Auto, true, &term));
        assert!(!color_with(ColorWhen::Auto, false, &term));
        assert!(!color_with(ColorWhen::Auto, true, &[("TERM", "dumb")]));
    }

    #[test]
    fn auto_respects_no_color_and_clicolor_force() {
        assert!(!color_with(ColorWhen::Auto, true, &[("NO_COLOR", "1")]));
        assert!(color_with(ColorWhen::Auto, true, &[("NO_COLOR", "")]));
        assert!(color_with(
            ColorWhen::Auto,
            false,
            &[("CLICOLOR_FORCE", "1")]
        ));
        assert!(!color_with(
            ColorWhen::Auto,
            false,
            &[("CLICOLOR_FORCE", "0")]
        ));
        assert!(!color_with(
            ColorWhen::Auto,
            false,
            &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")]
        ));
    }

    #[test]
    fn always_and_never_override_environment() {
        assert!(color_with(ColorWhen::Always, false, &[("NO_COLOR", "1")]));
        assert!(!color_with(
            ColorWhen::Never,
            true,
            &[("CLICOLOR_FORCE", "1")]
        ));
    }

    fn input(text: Option<&str>, raw: bool, stdin: &[u8]) -> Vec<u8> {
        read_input(text, raw, &mut &stdin[..]).unwrap()
    }

    #[test]
    fn stdin_loses_exactly_one_trailing_newline() {
        let cases: &[(&[u8], &[u8])] = &[
            (b"a\n", b"a"),
            (b"a\r\n", b"a"),
            (b"a\n\n", b"a\n"),
            (b"a\r", b"a\r"),
            (b"a\n\r", b"a\n\r"),
            (b"\n", b""),
            (b"\r\n", b""),
            (b"", b""),
            (b"multi\nline\n", b"multi\nline"),
        ];
        for (stdin, expected) in cases {
            assert_eq!(&input(None, false, stdin), expected, "stdin {:?}", stdin);
            assert_eq!(&input(Some("-"), false, stdin), expected);
            assert_eq!(&input(None, true, stdin), stdin, "raw keeps {:?}", stdin);
        }
    }

    #[test]
    fn text_argument_is_used_verbatim() {
        assert_eq!(input(Some("foo"), false, b"ignored"), b"foo");
        assert_eq!(input(Some("a\n"), false, b""), b"a\n");
        assert_eq!(input(Some("--"), false, b"stdin"), b"--");
        assert_eq!(input(Some(""), false, b"stdin"), b"");
    }

    #[test]
    fn read_errors_are_returned() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("boom"))
            }
        }
        assert!(read_input(None, false, &mut Broken).is_err());
        assert!(read_input(Some("x"), false, &mut Broken).is_ok());
    }

    #[test]
    fn capacity_matches_the_spec_for_each_mode() {
        // Version 40 at ECC level L; each limit only holds if the right mode was chosen.
        let cases = [("1", 7089), ("A", 4296), ("a", 2953)];
        for (unit, max) in cases {
            assert!(
                encode(unit.repeat(max).as_bytes()).is_ok(),
                "{} x {}",
                unit,
                max
            );
            assert!(
                encode(unit.repeat(max + 1).as_bytes()).is_err(),
                "{} x {}",
                unit,
                max + 1
            );
        }
    }

    #[test]
    fn capacity_counts_utf8_bytes() {
        assert!(encode("é".repeat(1476).as_bytes()).is_ok());
        assert!(encode("é".repeat(1477).as_bytes()).is_err());
        assert!(encode(&testutil::bytes(1, 2953)).is_ok());
        assert!(encode(&testutil::bytes(1, 2954)).is_err());
    }

    #[test]
    fn too_long_error_names_the_input_size() {
        let err = encode(&[b'a'; 3000]).err().unwrap();
        assert!(err.contains("3000 bytes"), "{}", err);
        assert!(err.contains("too long"), "{}", err);
    }

    #[test]
    fn small_inputs_get_boosted_error_correction() {
        let qr = encode(b"foobar").unwrap();
        assert_eq!(qr.version(), Version::new(1));
        assert_eq!(qr.error_correction_level(), QrCodeEcc::High);
        assert_eq!(encode(b"").unwrap().version(), Version::new(1));
        assert!(encode(&[0xff, 0xfe, 0x00]).is_ok());
    }

    #[test]
    fn mask_is_chosen_per_input() {
        // A fixed mask (0.1.0 always used mask 2) would show up as a single value.
        let masks: std::collections::BTreeSet<u8> = (0..64)
            .map(|seed| encode(&testutil::bytes(seed, 20)).unwrap().mask().value())
            .collect();
        assert!(masks.len() >= 4, "masks used: {:?}", masks);
    }

    #[test]
    fn boosting_never_grows_the_symbol() {
        for len in (0..=600).step_by(7) {
            let data = testutil::bytes(len as u64, len);
            let boosted = encode(&data).unwrap();
            let segs = [QrSegment::make_bytes(&data)];
            let plain = QrCode::encode_segments_advanced(
                &segs,
                QrCodeEcc::Low,
                Version::MIN,
                Version::MAX,
                None,
                false,
            )
            .unwrap();
            assert_eq!(boosted.version(), plain.version(), "len {}", len);
            assert!(boosted.error_correction_level() >= QrCodeEcc::Low);
        }
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("qrgen").chain(args.iter().copied()))
    }

    #[test]
    fn argument_defaults() {
        let cli = parse(&[]).unwrap();
        assert_eq!(cli.text, None);
        assert_eq!(cli.margin, 4);
        assert_eq!(cli.color, ColorWhen::Auto);
        assert!(!cli.small && !cli.svg && !cli.raw && !cli.invert);
    }

    #[test]
    fn margin_must_fit_a_byte() {
        assert_eq!(parse(&["-m", "0"]).unwrap().margin, 0);
        assert_eq!(parse(&["--margin=255"]).unwrap().margin, 255);
        for bad in [
            &["-m", "256"][..],
            &["--margin=-1"],
            &["-m", "x"],
            &["-m", ""],
        ] {
            let err = parse(bad).expect_err("should be rejected");
            assert_eq!(err.kind(), ErrorKind::ValueValidation, "{:?}", bad);
        }
    }

    #[test]
    fn conflicting_flags_are_rejected() {
        for args in [&["--svg", "--small"][..], &["--svg", "--invert"]] {
            let err = parse(args).expect_err("should conflict");
            assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{:?}", args);
        }
        assert!(parse(&["--small", "--invert", "--raw"]).is_ok());
    }

    #[test]
    fn color_values() {
        for (value, expected) in [
            ("auto", ColorWhen::Auto),
            ("always", ColorWhen::Always),
            ("never", ColorWhen::Never),
        ] {
            assert_eq!(parse(&["--color", value]).unwrap().color, expected);
        }
        assert_eq!(
            parse(&["--color", "sometimes"]).err().unwrap().kind(),
            ErrorKind::InvalidValue
        );
    }

    #[test]
    fn text_argument_forms() {
        assert_eq!(parse(&["-"]).unwrap().text.as_deref(), Some("-"));
        assert_eq!(
            parse(&["-s", "hello world"]).unwrap().text.as_deref(),
            Some("hello world")
        );
        assert_eq!(parse(&["--", "-m"]).unwrap().text.as_deref(), Some("-m"));
        assert_eq!(
            parse(&["a", "b"]).err().unwrap().kind(),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    fn help_and_version_short_circuit() {
        assert_eq!(parse(&["-h"]).err().unwrap().kind(), ErrorKind::DisplayHelp);
        assert_eq!(
            parse(&["-V"]).err().unwrap().kind(),
            ErrorKind::DisplayVersion
        );
        Cli::command().debug_assert();
    }

    #[test]
    fn color_decision_table() {
        for when in [ColorWhen::Auto, ColorWhen::Always, ColorWhen::Never] {
            for tty in [false, true] {
                let expected = match when {
                    ColorWhen::Always => true,
                    ColorWhen::Never => false,
                    ColorWhen::Auto => tty,
                };
                assert_eq!(
                    color_with(when, tty, &[]),
                    expected,
                    "{:?} tty={}",
                    when,
                    tty
                );
            }
        }
        // TERM unset is not "dumb"; an empty CLICOLOR_FORCE doesn't force.
        assert!(color_with(ColorWhen::Auto, true, &[]));
        assert!(!color_with(
            ColorWhen::Auto,
            false,
            &[("CLICOLOR_FORCE", "")]
        ));
        assert!(color_with(
            ColorWhen::Auto,
            false,
            &[("CLICOLOR_FORCE", "yes")]
        ));
    }

    #[test]
    fn readme_usage_matches_help() {
        let readme = include_str!("../README.md");
        let start = readme.find("## Usage\n\n````\n").expect("usage section") + 15;
        let block = &readme[start..start + readme[start..].find("\n````").unwrap()];
        let help = Cli::command().render_help().to_string();
        assert_eq!(
            block,
            help.trim_end(),
            "README usage is out of date; paste `qrgen --help`"
        );
    }
}
