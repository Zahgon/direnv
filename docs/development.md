# Development

Setup a Rust environment https://www.rust-lang.org/tools/install

> Rust >= 1.88 (stable) is required

Clone the project:

    $ git clone git@github.com:direnv/direnv.git

Build by just typing make:

    $ cd direnv
    $ make

Test the projects:

    $ make test

To install to /usr/local:

    $ make install

Or to a different location like `~/.local`:

    $ make install PREFIX=~/.local

## Lints

`make test` runs `cargo clippy --all-targets -- -D warnings` and
`cargo fmt --check` alongside the test suite; both must be clean.
