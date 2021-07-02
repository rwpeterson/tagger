# tagtools

A set of time tag analysis tools in Rust

## How to install

Clone the repository and install locally:

    git clone https://git.sr.ht/~rwp/tagtools-rs
    cd tagtools-rs
    cargo install --path .

If you do not have
a Rust toolchain and `cargo` installed,
see the [rustup](https://rustup.rs/) website.

## Development

If you want to hack on the crate,
recall that `cargo build` produces debug binaries.
For performance-intensive tasks,
it's better to experiment with release binaries:

    cargo build --release
    ./target/release/tsv2hist 1 3 14 -64 64 /path/to/my/tags.tsv

## Included tools

### `tsv2hist`

    tsv2hist win ch_a ch_b min_delay max_delay [tags.tsv]

Take a standard csv file
(really, *tab*-separated values
of channel and time
by our convention),
and output newline-separated histogram to stdout.
All window and delay parameters
are integers in units of the tag resolution.

For now, it seems faster
to pipe the tsv into `tsv2hist`,
rather than specifying a filename.
