# `tagger`

A set of time tag analysis and control code written in Rust. Programs
should work with both Windows and Linux (apart from some small command-line
programs that are Linux-only).

If you are new to Rust, see Getting Started at the bottom.

## Organization

This repository contains several libraries and applications organized
in one Cargo workspace. [Cargo][c] is Rust's package manager, and a
[workspace][w] is a set of multiple libraries or applications ("crates")
that are grouped together, e.g. so that they share common library code
in the same repository.

## End-user crates

### `streamer`

Server program that controls the time tagger. Clients can subscribe to
different types of data (raw time tags, counts in specified coincidence
patterns, etc) simultaneously.

#### Why client/server?

* A server on gigabit local network can stream tags to a client computer on the network
* Multiple different clients can subscribe simultaneously; the server takes the
  data everyone has subscribed to and parcels it out
* If you have particular needs with your client, you can write your own using
  the API specified in the capnp schema files.

### `tagtools`

Installs two helper utilities that convert from CSV to our compressed binary
format (`txt2tags`) and from compressed binary format to CSV (`tcat`).

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

### Install Rust toolchain

Get the Rust toolchain for your system at [rustup.rs][r].

### Install system dependencies

#### Zstandard (mandatory)

The [zstd][z] compression library is necessary.

#### capnproto (optional)

Install [capnp][p] if using your own code (e.g. Python) to use my APIs.

#### Proprietary vendor libraries (optional: time tagger control only)

Download the [vendor libraries][q] to copy over in a later step.

Note: Because these libraries cannot be freely distributed by me, the entire installation
process is somewhat complicated. Sorry. If you are not doing time tagger control at all
(or even just on a client-only computer), replace everything below with the one-liner
`cargo install --git https://git.sr.ht/~rwp/tagger <crate>` for each crate you want to (re)install.

### Installation

#### Clone the repository

Open a shell. Using [git][g], clone the repository locally and change directory into it

        cd /path/to/my/stuff
        git clone https://git.sr.ht/~rwp/tagger
        cd tagger

#### Copy over vendor libraries

If using for time tagger control, download and copy over [proprietary vendor libraries][q]

        mkdir -p lib
        cp /path/to/CTimeTagLib.lib lib  # Windows library
        cp /path/to/libtimetag64.so lib  # Linux library

#### Compile and install the crates you want

Pick which crates you want to install:

        cargo install --path ./<crate>
        cargo install --path ./streamer # for example
        cargo install --path ./tagview # etc...

Installed crates will then be available in your shell's PATH, e.g. just run `tagview`/`tagview.exe`

#### Update/uninstall

If you need to update, pull the changes via git and then reinstall everything

        cd /path/to/tagger
        git pull
        cargo install --path ./<crate>
        cargo install --path ./streamer # for example

To uninstall, simply

        cargo uninstall <crate>


[c]: https://doc.rust-lang.org/cargo/
[g]: https://git-scm.com/
[p]: https://capnproto.org/
[q]: https://uqdevices.com/products/
[r]: https://rustup.rs/
[w]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
[z]: https://facebook.github.io/zstd/