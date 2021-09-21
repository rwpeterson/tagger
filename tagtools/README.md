# tagtools

A set of time tag analysis tools in Rust

## End-user applications for working with the binary tags format

These work on Unix, not Windows (since Windows chokes on non-UTF8 bytes in stdin/stdout)

### `tcat`

Convert compressed binary format to tab-separated values

Example:

    tcat mydata.tags.zst > mydata.txt

reads `mydata.tags.zst`, decompresses it, parses the binary format, and writes it to
standard output as tab-separated values (the `> mydata.tags.zst` directs output to this
file instead of the terminal).

### `txt2tags`

Convert tab-separated data to the compressed binary format.

Example:

    txt2tags mydata.txt > mydata.tags.zst

reads `mydata.txt` (tab-separated values of channel and timestamp), and writes it to
standard output (the `> mydata.tags.zst` directs output to this file instead of the
terminal).

## Library

To view the modules related to (de)serialization, bit manipulation of coincidence
patterns, and analysis routines, it's simplest to use Cargo's built-in documentation
builder

    cargo doc --open

This builds documentation for all dependencies in the workspace as well, so search
for "tagtools" in the browser.

## Tests

This crate has several unit tests and integration tests to help ensure reliability.
The integration tests are in `./tests`, while unit tests are in their respective
module's source file.

Some integration tests, like those that ensure several GiB of tags are correctly
serialized and deserialized, are very memory-intensive and time-consuming to
run. They are decorated with `#[ignore]`, and are not run by default. One can
opt-in to testing them along with the others by passing the `--ignored` flag,
and (recommended) the `--test-threads=1` flag to avoid using all system memory
by running several in parallel:

```sh
cargo test -p tagtools                               # normal
cargo test -p tagtools -- --ignored --test-threads=1 # runs "ignored" tests too
```