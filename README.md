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

* A server on gigabit local network can stream tags to a separate client computer on the network
* Multiple different clients can subscribe simultaneously, e.g.
  - a GUI that displays current count rates
  - a script that saves desired data, easily integrated into control code
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

#### C++ toolchain (time tagger control only)

Because the tagger control code is a C++ library, you need a C++ compiler too.

##### Windows

Download and install the [Visual Studio with C++][m]
Community Edition. (Note: "Visual Studio", not "Visual Studio Code"). You shouldn't need to
ever use Visual Studio directly; Rust will just use the MSVC compiler on its own as needed.

##### Linux

You need gcc (clang not currently supported due to C++20 features I use).

For Debian/Ubuntu:

    sudo apt install build-essential

Other distributions will vary.


#### Proprietary vendor libraries (time tagger control only)

Download the [vendor libraries][q] to copy over in a later step.

Note: Because these libraries cannot be freely distributed by me, the entire installation
process is somewhat complicated. Sorry. If you are not doing time tagger control at all
(or even just on a client-only computer), replace everything below with the one-liner
`cargo install --git https://git.sr.ht/~rwp/tagger <crate>` for each crate you want to (re)install.

#### capnproto (optional; you are probably not doing this)

Install [capnp][p] if using your own non-Rust (e.g. Python) code to use the capnp APIs,
or if you are modifying/extending the schema and need to regenerate Rust code.

### Installation

#### Clone the repository

These instructions are for Linux. In Windows, make directories and copy over files as indicated.

Open a shell. Using [git][g], clone the repository locally and change directory into it

        cd ~
        mkdir git
        cd git
        git clone https://git.sr.ht/~rwp/tagger
        cd tagger

The repository is now located in `/home/<user>/git/tagger`. All further commands are assumed to
be in this directory (or whereever else you put it).

#### Copy over vendor libraries

If using for time tagger control, download and copy over [proprietary vendor libraries][q]

        mkdir -p lib
        cp /path/to/CTimeTagLib.lib lib  # Windows library
        cp /path/to/libtimetag64.so lib  # Linux library

#### Compile and install the crates you want

Pick which crates you want to install:

        cargo install --path ./<crate>
        cargo install --path ./streamer # for example
        cargo install --path ./tagview  # etc...

Installed crates will then be available in your shell's PATH, e.g. just run `tagview`/`tagview.exe`

#### Update/uninstall

If you need to update, pull the changes via git and then reinstall everything

        cd ~/git/tagger
        git pull
        cargo install --path ./<crate>
        cargo install --path ./streamer # for example

To uninstall, simply

        cargo uninstall <crate>

## License, commit ID, and data availability

Because you have a local copy of all the source code, you can keep track of the
version you are using. Just use git to check:

    git rev-parse HEAD
    # 98e1ccc96388242c3a4ddccdc5b2866c842cb22e

This is a "commit ID": a hash of the repository's current contents and its entire history.
This specifies the exact version you are using. Because it is long, you will often see people
refer to just the first seven hex digits, here `98e1ccc`. To meet data availability
requirements, you should specify both the repository and the commit ID you used. There doesn't
yet seem to be a standard way to cite a repository. Try something like this:

```text
Time tag analysis tools. Commit 98e1ccc. https://git.sr.ht/~rwp/tagger
```

Later I may mirror the repository on a host that specializes in long-term data preservation.
At that time I'll update this document. There should be the possibility of associating a DOI
with this repository (or even an individual commit).

The code is released under the MIT License.


[c]: https://doc.rust-lang.org/cargo/
[g]: https://git-scm.com/
[m]: https://visualstudio.microsoft.com/downloads/
[p]: https://capnproto.org/
[q]: https://uqdevices.com/products/
[r]: https://rustup.rs/
[w]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
[z]: https://facebook.github.io/zstd/