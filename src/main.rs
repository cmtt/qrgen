extern crate clap;
extern crate qrcodegen;
use clap::{App, Arg};
use qrcodegen::{Mask, QrCode, QrCodeEcc, QrCode_MAX_VERSION, QrCode_MIN_VERSION, QrSegment};
use std::io::Read;

const FULL_BLOCK: char = '\u{2588}'; //  █
const LOWER_HALF_BLOCK: char = '\u{2584}'; //  ▄
const UPPER_HALF_BLOCK: char = '\u{2580}'; //  ▀
const SPACE: char = '\u{20}'; //

fn main() {
    let matches = App::new("qrgen")
        .version("1.0")
        .author("Matthias Thoemmes <thoemmes@gmail.com>")
        .about("Renders input from stdin as QR codes on ANSI terminals.")
        .arg(
            Arg::with_name("small")
                .help("Outputs a smaller code.")
                .long("small")
                .short("s")
                .long("small"),
        )
        .arg(
            Arg::with_name("margin")
                .help("Margin size")
                .long("margin")
                .value_name("margin")
                .short("m"),
        )
        .arg(
            Arg::with_name("svg")
                .long("svg")
                .help("Outputs a SVG image"),
        )
        .get_matches();

    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();

    let margin = matches.value_of("margin").unwrap_or("2");
    let margin: i32 = margin.parse::<i32>().unwrap();

    let chrs: Vec<char> = buffer.chars().collect();
    let segs = QrSegment::make_segments(&chrs);
    let qr = QrCode::encode_segments_advanced(
        &segs,
        QrCodeEcc::Low,
        QrCode_MIN_VERSION,
        QrCode_MAX_VERSION,
        Some(Mask::new(2)),
        false,
    )
    .unwrap();

    if matches.is_present("svg") {
        print!("{}", qr.to_svg_string(margin));
    } else {
        render_qr_code(&qr, matches.is_present("small"), margin);
    }
}

fn render_qr_code(qr: &QrCode, small: bool, margin: i32) {
    let block_size = if small { 1 } else { 2 };
    let step = if small { 2 } else { 1 };
    for y in (-margin..qr.size() + margin).step_by(step) {
        for x in -margin..qr.size() + margin {
            let p0 = qr.get_module(x, y);
            let c = if small {
                match (p0, qr.get_module(x, y + 1)) {
                    (true, true) => FULL_BLOCK,
                    (true, false) => UPPER_HALF_BLOCK,
                    (false, true) => LOWER_HALF_BLOCK,
                    _ => SPACE,
                }
            } else if p0 {
                FULL_BLOCK
            } else {
                SPACE
            };
            for _ in 0..block_size {
                print!("{}", c);
            }
        }
        println!();
    }
}
