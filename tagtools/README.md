# tagtools

A set of time tag analysis tools in Rust

## Tests

Some integration tests, like those that ensure large amounts of tags
(4 GiB and 16 GiB) are correctly serialized and deserialized, are very
time-consuming to run. They are decorated with `#[ignore]`, and are not
run by default. One can opt-in to testing them along with the others
like so:

```sh
cargo test -p tagtools                # normal
cargo test -p tagtools -- --ignored   # runs "ignored" tests too
```