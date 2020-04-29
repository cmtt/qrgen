# qrgen

Renders input from stdin as QR codes on ANSI terminals.

## Installation

It is recommended to install Rust stable using [Rustup](https://rustup.rs/). In order to build and install `qrgen`, run the following command:

````bash
cargo install --path .
````

## Usage
````
USAGE:
    qrgen [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -s, --small      Outputs a smaller code.
        --svg        Outputs a SVG image
    -V, --version    Prints version information

OPTIONS:
    -m, --margin <margin>    Margin size
````

#### Display QR code in terminal

````
$ echo "foobar" | qrgen


    ██████████████      ██████  ██████████████
    ██          ██  ██████  ██  ██          ██
    ██  ██████  ██      ██████  ██  ██████  ██
    ██  ██████  ██  ████    ██  ██  ██████  ██
    ██  ██████  ██    ██    ██  ██  ██████  ██
    ██          ██  ██    ██    ██          ██
    ██████████████  ██  ██  ██  ██████████████
                      ██
    ██████████  ██████    ██  ██  ██  ██  ██
    ██  ████████      ██  ████████    ██  ██
    ██      ████████        ██  ████
    ████  ██            ██████████    ██  ██
        ██████  ██      ██  ██    ██    ██
                    ██  ██  ██    ██    ██
    ██████████████  ████  ██  ██    ██████████
    ██          ██    ██          ████  ██  ██
    ██  ██████  ██  ████████  ██    ██████████
    ██  ██████  ██  ██████████████    ██
    ██  ██████  ██  ██      ██  ████
    ██          ██  ██    ████████    ██
    ██████████████  ██      ██    ██    ██



````

#### Display QR code in terminal (small)
````
$ echo "foobar" | qrgen --small

  █▀▀▀▀▀█ ▄▄█▀█ █▀▀▀▀▀█
  █ ███ █ ▄▄▀▀█ █ ███ █
  █ ▀▀▀ █ ▄▀ ▄▀ █ ▀▀▀ █
  ▀▀▀▀▀▀▀ ▀▄▀ ▀ ▀▀▀▀▀▀▀
  ▀▀▀▀▀ ▀▀█  █▄█▄█ █ ▀▄
   █▀▀█ ▀▄▀█▄▄█▄██▄█▄▀▄
  ▀ ▀ ▀▀▀▀█▀  █  █▀▄ ▀▄
  █▀▀▀▀▀█ ▀▀█▀ ▀ █▄█▄█▄
  █ ███ █ █▀▀█▄█▄▀▄ ▄▀
  █ ▀▀▀ █ █▀█▄█▄█  ▄▄
  ▀▀▀▀▀▀▀ ▀   ▀  ▀   ▀

````

#### Display QR code as image

The following command generates and displays a generated QR code using [ImageMagick](ImageMagick)'s `display` command.

````
$ echo "foobar" | qrgen --svg | display -antialias -resample 400%  -
````

# License

MIT
