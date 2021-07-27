# tagtools

A set of time tag analysis tools in Rust

## Tests

Some integration tests, like those that ensure several GiB of tags are correctly
serialized and deserialized, are very memory-intensive and time-consuming to
run. They are decorated with `#[ignore]`, and are not run by default. One can
opt-in to testing them along with the others by passing the `--ignored` flag,
and (recommended) the `--test-threads=1` flag to avoid using all system memory:

```sh
cargo test -p tagtools                               # normal
cargo test -p tagtools -- --ignored --test-threads=1 # runs "ignored" tests too
```