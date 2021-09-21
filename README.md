# `tagger`

A set of time tag analysis and control code written in Rust. Programs
should work with both Windows and Linux (apart from some small command-line
programs that are Linux-only).

If you are new to Rust, see Getting Started at the bottom.

## Organization

This repository contains several libraries and applications organized
in one Cargo workspace. [Cargo][c] is Rust's package manager, and a
[workspace][w] is a set of multiple libraries or applications ("crates")
that are grouped together.

## End-user crates

### `streamer`

Server program that controls the time tagger. Clients can subscribe to
different types of data (raw time tags, counts in specified coincidence
patterns, etc) simultaneously. Why client/server? On the same computer
it's somewhat redundant, but this allows a server on gigabit local network
to stream tags to another computer. This keeps you from needing to manage
stored data on both your local lab computer controlling the experiment
and the remote computer next to the tagger/detectors: it can all stay on
the lab control computer.

### `tagtools`

Installs two helper utilities that convert from CSV to our compressed binary
format (`txt2tags`) and from compressed binary format to CSV (`tcat`)

### `tagview`

Client program to connect to `streamer`. Visual monitoring of subscribed
patterns and tagger status. TODO: Reads a file containing
a list of patterns/acquisition times to automate saving data.

## Libraries

### `tagger_capnp`

This crate stores schema files for our binary file formats, as well as the
generated Rust code to work with them.

Because the tagger can record time tags at a steady-state rate of ~10 MHz,
experiments can generate large amounts of data very quickly. This influences
both how data should be saved to disk for later analysis, and how the raw data
should be sent between different programs live during data-taking.

To efficiently store and work with this data, we use [Cap'n Proto][p] to serialize
it to a binary format. Given a schema file defining the data structure, Cap'n Proto
can programatically generate code to read and write this format in most major
languages.

### `tagtools`

`tagtools` covers tag serialization and analysis. This library is what reads and writes
our binary tag file format. It also has coincidence, g2, and other analysis routines
written in fast Rust, with integration tests to compare against known-good codes.

### `timetag`

`timetag` is a Rust wrapper for the `CTimeTag` and `CLogic` vendor C++ classes
for interfacing with the [UQDevices Logic16][q] time tagger. This library lets
other Rust program directly call the C++ code to control the tagger. There is
a small amount of overhead, but the library can still saturate the ~11 MHz data
transfer rate of the USB 2.0 interface.

## Getting started

1. Install the Rust toolchain from [rustup.rs][r]
2. Install system dependencies
  * [capnp][p] (optional: if using your own scripts in e.g. Python to read binary format)
  * [zstd][z] (necessary)
3. Clone the repository using [`git`][g] and change directory into it

        git clone https://git.sr.ht/~rwp/tagger
        cd tagger

4. If using for time tagger control, copy over proprietary vendor libraries

        mkdir lib
        cp /path/to/CTimeTagLib.lib lib  # Windows library
        cp /path/to/libtimetag64.so lib  # Linux library

4. Compile and install the crates you want (they will then be available in your shell's PATH)

        cargo install --path ./tagtools
        cargo install --path ./streamer
        cargo install --path ./tagview


[c]: https://doc.rust-lang.org/cargo/
[g]: https://git-scm.com/
[p]: https://capnproto.org/
[q]: https://uqdevices.com/products/
[r]: https://rustup.rs/
[w]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
[z]: https://facebook.github.io/zstd/