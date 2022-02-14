# tagtools

A set of time tag analysis tools in Rust

## End-user applications for working with the [binary tags format](doc/tags_format.md)

### `tcat`

Convert compressed binary format to tab-separated values

```sh
tcat mydata.tags.zst > mydata.txt
```

reads `mydata.tags.zst`, decompresses it, parses the binary format, and writes it to
standard output as tab-separated values (the `> mydata.txt` directs output to this
file instead of the terminal).

### `txt2tags`

Convert tab-separated data to the compressed binary format.

```sh
txt2tags mydata.txt > mydata.tags.zst
```

reads `mydata.txt` (tab-separated values of channel and timestamp), and writes it to
standard output (the `> mydata.tags.zst` directs output to this file instead of the
terminal).

Note that on Windows, you must use `-o` to specify the output file (rather than use stdout),
e.g. `txt2tags mydata.txt -o mydata.tags.zst`, because due to a stdlib limitation, Rust cannot
emit non-UTF8 bytes to standard output on Windows platforms.

### "I want to read your binary tags format, but I refuse to use your code"

You can use the [`capnp`][cpt] program to decode the binary to a human-readable format:

```sh
zstd -d mydata.tags.zst # decompresses to mydata.tags
capnp decode tags.capnp Tags < mydata.tags > mydata.txt
```

## End-user applications for working with the [experiment runfile format](src/cfg.rs)

This format, defined in `tagtools::cfg::Run`, is a JSON file that can either specify
the data to be taken in an experiment, or record the data that was taken. See the
[example runfile](contrib/runfile_example.json).

### `checkrun`

You can verify that your runfile parses correctly into a Run object

```sh
checkrun myrunfile.json
```

Exiting with no output means that the runfile parsed correctly.

## Library API

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

## Benchmarks

[Criterion][crit] benches are provided to monitor the performance of the following components:

- `BitOps` trait: bitwise set/clear/toggle/change/check operations, confirming
  no overhead vs writing the literal bitwise operations
- Coincidence algorithms: set intersection and single-delay histogram
- Serialization: packed/unpacked Cap'n Proto message and zstd compression levels,
  which inform default choices

After running `cargo bench`, check out [the report](../target/criterion/report/index.html).

[cpt]: https://capnproto.org/capnp-tool.html#decoding-messages
[crit]: https://bheisler.github.io/criterion.rs/book/index.html