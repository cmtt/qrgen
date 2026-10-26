# qrgen

Renders text as QR codes on Unicode terminals or as SVG.

## Installation

It is recommended to install Rust stable (1.85 or newer) using [Rustup](https://rustup.rs/). In order to build and install `qrgen` from [crates.io](https://crates.io/crates/qrgen), run the following command:

````bash
cargo install qrgen
````

To install from a checkout of this repository instead, run `cargo install --path .`.

## Usage

````
Renders text as QR codes on Unicode terminals or as SVG.

Usage: qrgen [OPTIONS] [TEXT]

Arguments:
  [TEXT]  Text to encode. Reads stdin if omitted or "-"

Options:
  -s, --small             Half the height using ▀▄█ glyphs (needs a font that draws them exactly)
  -m, --margin <MODULES>  Quiet-zone width in modules [default: 4]
      --svg               Write an SVG image instead
      --raw               Keep a trailing newline read from stdin
      --color <WHEN>      Paint black on white regardless of theme (auto: terminals only, honours NO_COLOR) [default: auto] [possible values: auto, always, never]
  -i, --invert            Swap dark and light modules (for dark themes with --color never)
  -h, --help              Print help
  -V, --version           Print version
````

Text is taken from the `TEXT` argument or, if omitted, read from stdin (`echo … | qrgen`, `qrgen < file`). Run without either in a terminal, `qrgen` prints usage; use `qrgen -` to type or paste the input instead, ending with Ctrl-D. A single trailing newline on stdin (as added by `echo`) is not encoded; pass `--raw` to keep it. Input that is not valid UTF-8 is encoded as raw bytes.

The examples below are the plain-text form, see the troubleshooting section below for details in case of unexpected rendered output.

### Display QR code in terminal

````
$ qrgen --margin 2 foobar


    ██████████████    ██████    ██████████████
    ██          ██              ██          ██
    ██  ██████  ██      ██  ██  ██  ██████  ██
    ██  ██████  ██      ██  ██  ██  ██████  ██
    ██  ██████  ██  ██    ████  ██  ██████  ██
    ██          ██      ██████  ██          ██
    ██████████████  ██  ██  ██  ██████████████
                    ██    ██
        ████    ██████    ██  ████  ██
          ██████      ██████  ██████  ██    ██
      ██████    ██████  ████  ████████  ██████
    ██    ██  ██    ██████  ██████    ██  ████
    ██    ██████████████            ██    ██
                    ██  ████            ██
    ██████████████  ██      ██      ██████
    ██          ██      ████  ██  ████████  ██
    ██  ██████  ██      ██    ████    ████████
    ██  ██████  ██  ██████  ██  ████      ██
    ██  ██████  ██  ████      ████
    ██          ██      ████████████        ██
    ██████████████    ████          ██  ██


````

### Display QR code in terminal (small)

Packs two module rows into one line using `▀▄█`, so it's half the height. It's exact in terminals that draw block characters themselves (for example kitty, Ghostty, WezTerm, Alacritty, and VTE-based terminals such as GNOME Terminal). Terminals that take the glyphs from the font, such as macOS Terminal.app, size them to the font's em box, which is about 16% shorter than a line, so the code comes out slightly distorted. Use the default mode there.

````
$ echo "foobar" | qrgen --small


    █▀▀▀▀▀█  ▀▀▀  █▀▀▀▀▀█
    █ ███ █   █ █ █ ███ █
    █ ▀▀▀ █ ▀ ▄██ █ ▀▀▀ █
    ▀▀▀▀▀▀▀ █ ▀▄▀ ▀▀▀▀▀▀▀
      ▀█▄▄▀▀▀▄▄█ ██▄▀▄  ▄
    ▄▀▀█ ▄▀▀█▄█▀▄██▀▀▄▀██
    ▀  ▀▀▀▀▀█▀▄▄    ▀ ▄▀
    █▀▀▀▀▀█ ▀ ▄▄▀▄ ▄███ ▄
    █ ███ █ ▄▄█ ▄▀█▄ ▀▀█▀
    █ ▀▀▀ █ ▀▀▄▄▄██▄    ▄
    ▀▀▀▀▀▀▀  ▀▀     ▀ ▀


````

### Display QR code as image

The following command generates and displays a generated QR code using [ImageMagick](https://imagemagick.org/)'s `display` command.

````
$ echo "foobar" | qrgen --svg | display -antialias -resample 500% -define filter:support=0 -
````

## Troubleshooting scanning

- **Distorted code with `--small`**: the terminal draws `▀▄█` from the font, and they don't fill the cell (e.g. Terminal.app with SF Mono or Menlo). Use the default mode, which needs no glyphs when colours are on.
- **No colours** (`--color never`, `NO_COLOR`, or piped output): the code is drawn in your theme's text colour, so on a dark theme it comes out inverted (light modules on a dark margin), which some scanners reject. Add `--invert` in that case.
- **Code doesn't fit**: if it wraps or scrolls, it can't be scanned. Enlarge the window, reduce the margin (`-m 2`), or try `--small` if your font draws block glyphs exactly.

## Utilizing colors for rendering

When writing to a terminal, `qrgen` paints the code in pure black and white (256-colour indices 16 and 231), so it reads the same on light and dark themes. In the default mode every module is two spaces with a black or white background, so no glyphs are involved and the result is exact in any font. Piped or redirected output stays plain text. On Windows, auto mode stays plain because the legacy console doesn't interpret the colour codes; use `--color always` in Windows Terminal. Use `--color always|never` to override, or set `NO_COLOR` (disables) / `CLICOLOR_FORCE=1` (enables in auto mode). 

# License

MIT, see [LICENSE](LICENSE).
