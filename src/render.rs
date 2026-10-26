use qrcodegen::QrCode;
use std::io::{self, Write};

const FULL_BLOCK: char = '\u{2588}'; //  █
const LOWER_HALF_BLOCK: char = '\u{2584}'; //  ▄
const UPPER_HALF_BLOCK: char = '\u{2580}'; //  ▀
const SPACE: char = '\u{20}'; //

// Colour output uses exactly two colours from the 256-colour cube, black (16)
// and white (231). Unlike indices 0-15 they aren't remapped by terminal themes,
// so the code is dark-on-light on both light and dark themes.
const FG_BLACK: &str = "\x1b[38;5;16m";
const FG_WHITE: &str = "\x1b[38;5;231m";
const BG_BLACK: &str = "\x1b[48;5;16m";
const BG_WHITE: &str = "\x1b[48;5;231m";
const RESET: &str = "\x1b[0m";

/// Options for the terminal renderers.
pub struct TermOpts {
    /// Quiet-zone width in modules.
    pub margin: u8,
    /// Swap dark and light modules, quiet zone included.
    pub invert: bool,
    /// Paint with black and white instead of relying on the terminal's colours.
    pub color: bool,
}

// The quiet zone is never drawn explicitly: the renderers iterate from -margin
// to size + margin and rely on QrCode::get_module returning false (light) for
// coordinates outside the symbol.
fn dark(qr: &QrCode, x: i32, y: i32, invert: bool) -> bool {
    qr.get_module(x, y) ^ invert
}

/// A line of colour output that only emits escape codes when a colour changes.
struct ColorLine {
    text: String,
    fg: Option<bool>,
    bg: Option<bool>,
}

impl ColorLine {
    fn new() -> Self {
        ColorLine {
            text: String::new(),
            fg: None,
            bg: None,
        }
    }

    /// Appends `c` on a dark or light background; `fg` is only set for glyphs.
    fn push(&mut self, c: char, fg: Option<bool>, bg: bool) {
        if let Some(fg) = fg {
            if self.fg != Some(fg) {
                self.text.push_str(if fg { FG_BLACK } else { FG_WHITE });
                self.fg = Some(fg);
            }
        }
        if self.bg != Some(bg) {
            self.text.push_str(if bg { BG_BLACK } else { BG_WHITE });
            self.bg = Some(bg);
        }
        self.text.push(c);
    }

    // Resetting before the newline keeps the background colour from bleeding
    // into the rest of the line when the terminal scrolls.
    fn finish(self, out: &mut impl Write) -> io::Result<()> {
        writeln!(out, "{}{}", self.text, RESET)
    }
}

/// Renders two module rows per text line using half-block characters.
///
/// With colour, cells whose two modules match are a space painted with the
/// background colour, so solid areas have no glyph seams.
///
/// Mixed cells are `▄` with the top module as background, so the top half is
/// always exact and only the bottom half depends on the glyph. Terminals that
/// draw glyphs from the font (e.g. Terminal.app) size block elements to the
/// em box, which is shorter than the line: in SF Mono `▀` misses the top 15%
/// of the cell and spills 7% past the middle, while `▄` only falls 7% short
/// of the middle.
pub fn half_blocks(qr: &QrCode, opts: &TermOpts, out: &mut impl Write) -> io::Result<()> {
    let margin = i32::from(opts.margin);
    for y in (-margin..qr.size() + margin).step_by(2) {
        let cells = (-margin..qr.size() + margin)
            .map(|x| (dark(qr, x, y, opts.invert), dark(qr, x, y + 1, opts.invert)));
        if opts.color {
            let mut line = ColorLine::new();
            for (top, bottom) in cells {
                if top == bottom {
                    line.push(SPACE, None, bottom);
                } else {
                    line.push(LOWER_HALF_BLOCK, Some(bottom), top);
                }
            }
            line.finish(out)?;
        } else {
            let line: String = cells
                .map(|cell| match cell {
                    (true, true) => FULL_BLOCK,
                    (true, false) => UPPER_HALF_BLOCK,
                    (false, true) => LOWER_HALF_BLOCK,
                    (false, false) => SPACE,
                })
                .collect();
            writeln!(out, "{}", line)?;
        }
    }
    Ok(())
}

/// Renders one module row per text line, two characters per module.
///
/// With colour, every module is two spaces painted with the background colour,
/// so the output uses no glyphs and renders exactly in any font.
pub fn full_blocks(qr: &QrCode, opts: &TermOpts, out: &mut impl Write) -> io::Result<()> {
    let margin = i32::from(opts.margin);
    for y in -margin..qr.size() + margin {
        let modules = (-margin..qr.size() + margin).map(|x| dark(qr, x, y, opts.invert));
        if opts.color {
            let mut line = ColorLine::new();
            for d in modules {
                line.push(SPACE, None, d);
                line.push(SPACE, None, d);
            }
            line.finish(out)?;
        } else {
            let mut line = String::new();
            for d in modules {
                let c = if d { FULL_BLOCK } else { SPACE };
                line.push(c);
                line.push(c);
            }
            writeln!(out, "{}", line)?;
        }
    }
    Ok(())
}

/// Writes an SVG document with one unit per module.
///
/// Adapted from `QrCode::to_svg_string` in qrcodegen 1.6, which was removed
/// from the library in later versions.
pub fn svg(qr: &QrCode, margin: u8, out: &mut impl Write) -> io::Result<()> {
    let margin = i32::from(margin);
    let dimension = qr.size() + margin * 2;
    writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        out,
        "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">"
    )?;
    writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\" viewBox=\"0 0 {0} {0}\" stroke=\"none\" shape-rendering=\"crispEdges\">",
        dimension
    )?;
    writeln!(
        out,
        "\t<rect width=\"100%\" height=\"100%\" fill=\"#FFFFFF\"/>"
    )?;
    write!(out, "\t<path d=\"")?;
    let mut separator = "";
    for y in 0..qr.size() {
        for x in 0..qr.size() {
            if qr.get_module(x, y) {
                write!(out, "{}M{},{}h1v1h-1z", separator, x + margin, y + margin)?;
                separator = " ";
            }
        }
    }
    writeln!(out, "\" fill=\"#000000\"/>")?;
    writeln!(out, "</svg>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;
    use qrcodegen::{Mask, QrCodeEcc, QrSegment, Version};
    use std::collections::BTreeSet;

    type Renderer = fn(&QrCode, &TermOpts, &mut Vec<u8>) -> io::Result<()>;

    /// Each terminal renderer, with whether it packs two module rows per line.
    const RENDERERS: [(Renderer, bool); 2] = [(full_blocks, false), (half_blocks, true)];

    const ECC_LEVELS: [QrCodeEcc; 4] = [
        QrCodeEcc::Low,
        QrCodeEcc::Medium,
        QrCodeEcc::Quartile,
        QrCodeEcc::High,
    ];

    fn code() -> QrCode {
        QrCode::encode_text("foobar", QrCodeEcc::Low).unwrap()
    }

    fn fixed(data: &[u8], version: u8, mask: u8, ecl: QrCodeEcc) -> QrCode {
        let version = Version::new(version);
        QrCode::encode_segments_advanced(
            &[QrSegment::make_bytes(data)],
            ecl,
            version,
            version,
            Some(Mask::new(mask)),
            false,
        )
        .unwrap()
    }

    /// Codes covering every version, every mask and ECC level, and random payloads.
    fn codes() -> Vec<QrCode> {
        let mut codes = Vec::new();
        for v in 1..=40u8 {
            let data = testutil::bytes(u64::from(v), usize::from(v));
            codes.push(fixed(&data, v, v % 8, ECC_LEVELS[usize::from(v) % 4]));
        }
        for mask in 0..8 {
            for ecl in ECC_LEVELS {
                codes.push(fixed(b"qrgen", 3, mask, ecl));
            }
        }
        for seed in 0..24u64 {
            let data = testutil::bytes(seed, (seed as usize * 97) % 1200);
            codes.push(QrCode::encode_binary(&data, QrCodeEcc::Low).unwrap());
        }
        codes
    }

    fn opts(margin: u8, invert: bool, color: bool) -> TermOpts {
        TermOpts {
            margin,
            invert,
            color,
        }
    }

    fn render_with(f: Renderer, qr: &QrCode, opts: &TermOpts) -> String {
        let mut out = Vec::new();
        f(qr, opts, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    fn render(f: Renderer, opts: &TermOpts) -> String {
        render_with(f, &code(), opts)
    }

    fn plain(f: Renderer, margin: u8) -> String {
        render(f, &opts(margin, false, false))
    }

    /// Splits a line into its escape codes and visible characters, in order.
    fn tokens(line: &str) -> Vec<Result<&str, char>> {
        let mut out = Vec::new();
        let mut rest = line;
        while let Some(c) = rest.chars().next() {
            if c == '\x1b' {
                let end = rest.find('m').expect("unterminated escape") + 1;
                out.push(Ok(&rest[..end]));
                rest = &rest[end..];
            } else {
                out.push(Err(c));
                rest = &rest[c.len_utf8()..];
            }
        }
        out
    }

    fn visible(line: &str) -> String {
        tokens(line).into_iter().filter_map(Result::err).collect()
    }

    /// Reads terminal output back into a module grid (true = dark), the way a
    /// terminal would paint it: glyph tables for plain output, and the
    /// foreground/background state for colour output.
    fn parse(out: &str, small: bool) -> Vec<Vec<bool>> {
        let mut grid = Vec::new();
        for line in out.lines() {
            let (mut fg, mut bg) = (None, None);
            let mut cells = Vec::new();
            for token in tokens(line) {
                match token {
                    Ok(FG_BLACK) => fg = Some(true),
                    Ok(FG_WHITE) => fg = Some(false),
                    Ok(BG_BLACK) => bg = Some(true),
                    Ok(BG_WHITE) => bg = Some(false),
                    Ok(RESET) => (fg, bg) = (None, None),
                    Ok(other) => panic!("unexpected escape {:?}", other),
                    Err(c) => cells.push(match (bg, c) {
                        (Some(bg), SPACE) => (bg, bg),
                        (Some(bg), LOWER_HALF_BLOCK) => (bg, fg.expect("glyph without fg")),
                        (Some(_), other) => panic!("unexpected glyph {:?} in colour", other),
                        (None, FULL_BLOCK) => (true, true),
                        (None, UPPER_HALF_BLOCK) => (true, false),
                        (None, LOWER_HALF_BLOCK) => (false, true),
                        (None, SPACE) => (false, false),
                        (None, other) => panic!("unexpected glyph {:?}", other),
                    }),
                }
            }
            if small {
                grid.push(cells.iter().map(|c| c.0).collect());
                grid.push(cells.iter().map(|c| c.1).collect());
            } else {
                assert!(cells.len() % 2 == 0, "odd line width");
                let modules: Vec<bool> = cells
                    .chunks(2)
                    .map(|pair| {
                        assert_eq!(pair[0], pair[1], "module halves differ");
                        assert_eq!(pair[0].0, pair[0].1, "full cell split vertically");
                        pair[0].0
                    })
                    .collect();
                grid.push(modules);
            }
        }
        grid
    }

    #[test]
    fn every_renderer_reproduces_every_module() {
        for qr in codes() {
            for (f, small) in RENDERERS {
                for color in [false, true] {
                    for invert in [false, true] {
                        for margin in [0u8, 1, 4, 7] {
                            let out = render_with(f, &qr, &opts(margin, invert, color));
                            let grid = parse(&out, small);
                            let m = i32::from(margin);
                            let side = (qr.size() + 2 * m) as usize;
                            let rows = if small { side + side % 2 } else { side };
                            assert_eq!(grid.len(), rows);
                            for (y, row) in grid.iter().enumerate() {
                                assert_eq!(row.len(), side);
                                for (x, &cell) in row.iter().enumerate() {
                                    let (qx, qy) = (x as i32 - m, y as i32 - m);
                                    assert!(
                                        cell == dark(&qr, qx, qy, invert),
                                        "v{} small={} color={} invert={} margin={} at ({}, {})",
                                        qr.version().value(),
                                        small,
                                        color,
                                        invert,
                                        margin,
                                        qx,
                                        qy
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn dimensions_for_every_version() {
        for v in 1..=40u8 {
            let qr = fixed(b"", v, 0, QrCodeEcc::Low);
            let margins: &[u8] = if v == 40 { &[0, 4, 255] } else { &[0, 4] };
            for &margin in margins {
                let side = qr.size() as usize + 2 * usize::from(margin);
                for color in [false, true] {
                    let o = opts(margin, false, color);
                    for (f, small) in RENDERERS {
                        let out = render_with(f, &qr, &o);
                        let lines: Vec<&str> = out.lines().collect();
                        let (rows, cols) = if small {
                            (side.div_ceil(2), side)
                        } else {
                            (side, 2 * side)
                        };
                        assert_eq!(lines.len(), rows, "v{} margin {}", v, margin);
                        assert!(lines.iter().all(|l| visible(l).chars().count() == cols));
                    }
                }
            }
        }
    }

    fn assert_finder(grid: &[Vec<bool>], left: usize, top: usize, invert: bool) {
        for dy in 0..7 {
            for dx in 0..7 {
                let ring = dx == 0 || dx == 6 || dy == 0 || dy == 6;
                let centre = (2..=4).contains(&dx) && (2..=4).contains(&dy);
                let expected = (ring || centre) ^ invert;
                assert_eq!(
                    grid[top + dy][left + dx],
                    expected,
                    "finder at ({}, {})",
                    left,
                    top
                );
            }
        }
    }

    #[test]
    fn finder_patterns_sit_in_three_corners() {
        for v in [1u8, 2, 7, 21, 40] {
            let qr = fixed(b"finder", v, 3, QrCodeEcc::Medium);
            let size = qr.size() as usize;
            for (f, small) in RENDERERS {
                for color in [false, true] {
                    for invert in [false, true] {
                        let margin = 3;
                        let grid = parse(&render_with(f, &qr, &opts(3, invert, color)), small);
                        let far = margin + size - 7;
                        assert_finder(&grid, margin, margin, invert);
                        assert_finder(&grid, far, margin, invert);
                        assert_finder(&grid, margin, far, invert);
                        // Just outside the top-left finder is quiet zone.
                        assert_eq!(grid[margin - 1][margin - 1], invert, "quiet zone");
                    }
                }
            }
        }
    }

    #[test]
    fn color_escapes_are_minimal_and_balanced() {
        let colours = [FG_BLACK, FG_WHITE, BG_BLACK, BG_WHITE];
        for qr in codes().iter().step_by(5) {
            for (f, _) in RENDERERS {
                for invert in [false, true] {
                    let out = render_with(f, qr, &opts(2, invert, true));
                    for line in out.lines() {
                        let toks = tokens(line);
                        assert!(matches!(toks.first(), Some(Ok(BG_BLACK | BG_WHITE))));
                        assert_eq!(toks.last(), Some(&Ok(RESET)));
                        let (mut fg, mut bg) = (None, None);
                        for (i, tok) in toks.iter().enumerate() {
                            let Ok(code) = *tok else { continue };
                            if code == RESET {
                                assert_eq!(i, toks.len() - 1, "reset mid-line");
                                continue;
                            }
                            assert!(colours.contains(&code), "{:?}", code);
                            let slot = if code.starts_with("\x1b[38") {
                                &mut fg
                            } else {
                                &mut bg
                            };
                            assert_ne!(*slot, Some(code), "redundant {:?}", code);
                            *slot = Some(code);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn zero_margin_starts_with_finder_pattern() {
        assert!(plain(half_blocks, 0).starts_with("█▀▀▀▀▀█"));
        assert!(plain(full_blocks, 0).starts_with("██████████████"));
    }

    #[test]
    fn invert_flips_every_module() {
        let normal = plain(half_blocks, 2);
        let inverted = render(half_blocks, &opts(2, true, false));
        let flip = |c| match c {
            FULL_BLOCK => SPACE,
            SPACE => FULL_BLOCK,
            UPPER_HALF_BLOCK => LOWER_HALF_BLOCK,
            LOWER_HALF_BLOCK => UPPER_HALF_BLOCK,
            other => other,
        };
        let flipped: String = normal.chars().map(flip).collect();
        // The last line's bottom half lies past the quiet zone and flips too.
        assert_eq!(inverted, flipped);
    }

    #[test]
    fn color_full_blocks_use_no_glyphs() {
        for qr in codes().iter().step_by(7) {
            let out = render_with(full_blocks, qr, &opts(2, false, true));
            assert!(out.lines().all(|l| visible(l).chars().all(|c| c == SPACE)));
        }
    }

    fn render_svg(qr: &QrCode, margin: u8) -> String {
        let mut out = Vec::new();
        svg(qr, margin, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn svg_paths_match_dark_modules() {
        for qr in codes().iter().step_by(3) {
            for margin in [0u8, 4, 255] {
                let out = render_svg(qr, margin);
                let m = i32::from(margin);
                let d = out.split("d=\"").nth(1).unwrap().split('"').next().unwrap();
                let drawn: BTreeSet<(i32, i32)> = d
                    .split(' ')
                    .map(|p| {
                        let xy = p
                            .strip_prefix('M')
                            .unwrap()
                            .strip_suffix("h1v1h-1z")
                            .unwrap();
                        let (x, y) = xy.split_once(',').unwrap();
                        (x.parse().unwrap(), y.parse().unwrap())
                    })
                    .collect();
                let expected: BTreeSet<(i32, i32)> = (0..qr.size())
                    .flat_map(|y| (0..qr.size()).map(move |x| (x, y)))
                    .filter(|&(x, y)| qr.get_module(x, y))
                    .map(|(x, y)| (x + m, y + m))
                    .collect();
                assert_eq!(drawn, expected, "margin {}", margin);
                assert_eq!(d.split(' ').count(), expected.len(), "duplicate paths");
            }
        }
    }

    #[test]
    fn svg_document_structure() {
        for margin in [0u8, 4, 255] {
            let out = render_svg(&code(), margin);
            let side = code().size() + 2 * i32::from(margin);
            assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
            assert!(out.contains(&format!("viewBox=\"0 0 {0} {0}\"", side)));
            assert!(out.contains("shape-rendering=\"crispEdges\""));
            assert_eq!(out.matches("<svg ").count(), 1);
            assert_eq!(out.matches("<path ").count(), 1);
            assert!(out.ends_with("</svg>\n"));
        }
    }

    /// A writer that fails with a broken pipe after accepting `left` bytes.
    struct FailAfter {
        left: usize,
    }

    impl Write for FailAfter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let n = buf.len().min(self.left);
            self.left -= n;
            Ok(n)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn write_errors_are_returned() {
        let qr = code();
        for left in [0, 10, 500] {
            for color in [false, true] {
                let o = opts(4, false, color);
                assert!(full_blocks(&qr, &o, &mut FailAfter { left }).is_err());
                assert!(half_blocks(&qr, &o, &mut FailAfter { left }).is_err());
            }
            assert!(svg(&qr, 4, &mut FailAfter { left }).is_err());
        }
    }
}
