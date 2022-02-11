# `tagger`

A set of time tag analysis and control code written in Rust. Programs
work with both Windows and Linux (x86_64 only).

The end-user applications are available as binaries, but can also
be compiled and ran from source. The libraries here can also be used
in your own programs.

If you are new to Rust, see Getting Started at the bottom.

## Organization

This repository contains several libraries and applications organized
in one Cargo workspace. [Cargo][c] is Rust's package manager, and a
[workspace][w] is a set of multiple libraries or applications ("crates")
that are grouped together, e.g. so that they share common library code
in the same repository.

## High-level overview

All applications support `--help` to display basic usage and command-line arguments

### Data collection and instrument control

- `tagstream`: Server program that controls the time tagger and provides tags and
  count information to clients
- `tagview`: Interactive client program that displays current count rates,
  controls input delays and thresholds, and so on
- `tagsave`: Automated program that takes a .json specification of the
  data you want to save, connects to a local or remote `tagstream` server
  to collect the data, then saves it as .json and (if requested) saves
  the raw tags in our compressed binary format alongside

### Analysis and processing

- `tcat`: decompresses and decodes our [compressed binary format](tagtools/doc/tags_format.md)
  to tab-separated values for use in other tools
- `txt2tags`: compresses tab-separated time tag data into our compressed binary
  format

## Why does instrument control and data collection use a client/server interface?

- A server on gigabit local network can stream tags to a separate client computer on the network
  + See [iperf](https://linux.die.net/man/1/iperf) for a command line tool to test your
    local network's speed
  + A client computer on a slower LAN (or remote over the internet) can stream summary
    statistics like pattern count rates, without raw tag data
- Multiple different clients can subscribe simultaneously, e.g.
  + a GUI that displays current count rates for interative use and monitoring
  + a script that saves desired data, easily integrated into control code
- If you have particular needs with your client, you can write your own using
  the API specified in the capnp schema files, without needing to reimplement
  the instrument control

## Libraries

### `tagger_capnp` ([readme](tagger_capnp/README.md))

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

### `tagtools` ([readme](tagtools/README.md))

`tagtools` covers tag serialization and analysis. This library is what reads and writes
our binary tag file format. It also has coincidence, g2, and other analysis routines
written in fast Rust, with integration tests to compare against known-good codes.

### `timetag` ([lib.rs](timetag/src/lib.rs))

`timetag` is a Rust wrapper for the `CTimeTag` and `CLogic` vendor C++ classes
for interfacing with the [UQDevices Logic16][q] time tagger. This library lets
other Rust program directly call the C++ code to control the tagger. There is
a small amount of overhead, but the library can still saturate the ~11 MHz data
transfer rate of the USB 2.0 interface.

## Schematic overview of components

```text
+-----------------+                   +----------------+
|   Time tagger   |                   |   CTimeTag.h   |
| vendor hardware |<---- USB 2.0 ---->| vendor library |
|     (FPGA)      |                   |     (C++)      |
+-----------------+                   +----------------+
                                           ^
   vendor                                  |
-----8<-----                               v
this project       +-----------------------------------+
                   |            taghelper.h            |
                   | Smart pointer/std::vector wrapper |
                   |              (C++)                |
                   +-----------------------------------+
                                          ^
                                          | CXX FFI
                                          v
+----------------------------+    +---------------------------+
|         tagstream          |    |         timetag           |
| Time tagger control server |<-->| Rust bindings for library |
|                            |    +---------------------------+
| async runtime: tokio       |
| RPC: Cap'n Proto           |
+----------------------------+            control computer
                         ^             ----------8<-----------
                         |             control comp. or remote
     tag_server.capnp    |
         RPC API         +-------------------+
                         v                   v
+------------------------------+    +------------------------------+
|           tagview            |    |          tagsave             |
| tui-rs terminal ui/dashboard |    | automated instrument control |
| interactive monitor/control  |    |     and data acquisition     |
+------------------------------+    +------------------------------+
     ^                                ^                     |
     | interactively tune delays,     | load specification  | save summary data, metadata
     | thresholds, etc.               | of data to take     | and raw tags
     v                                |                     +-----------------+
   +-------------+                 +-------------+          |                 |
   | myexpt.json |---------------->| myexpt.json |          v                 |
   +-------------+ finalize data   +-------------+     +------------------------------+
                   run parameters                      | 20220119T123501Z_myexpt.json |
                                                       +------------------------------+------+
                                                          | 20220119T123501Z_myexpt.tags.zst |
                                                          +----------------------------------+
```

## Concept of how data is saved

Programs that accept a configuration, e.g. `myexpt.json`, will save data in a
consistent way. The file stem of the config, e.g. `myexpt`, will be prepended
with a timestamp in the saved runfile, e.g. `20220119T123501Z_myexpt.json`.
For details of the format, see
[the documentation of the Rust code that reads the JSON format](./tagtools/src/cfg.rs).

### About timestamps and their representation

It is important to store date and time in a consistent format that will
not cause unanticipated errors. The most prescriptive standard in regular use
is RFC 3339, which is a profile of ISO 8601 (meaning a specific choice among
various representation options permitted by ISO 8601).

Sadly, a timestamp in a cross-platform filename cannot be RFC 3339 compliant as
it mandates a ':', which is an illegal character in Windows. However, it can be
ISO 8601 compliant if it uses no '-' or ':' separators. Further, we consistently
use UTC to avoid the implicit assumption of a timezone (which must be explicitly
denoted if used), and provide a standard that can be of use in the lab, e.g.
with instruments which store the time but do not automatically adjust for DST,
and are best left set to UTC.

The format is as follows, using the [`date(1)`](https://linux.die.net/man/1/date) format:
```
%Y%m%dT%H%M%SZ
20220119T123501Z
```
corresponding to the RFC-3339-compliant UTC time 2022-01-19T12:35:01Z,
a.k.a. 11:35 CET in Vienna.

Your programming language should have no trouble reading this timestamp using
its standard library (typically called "datetime").

## Getting started with development or installing from source

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

```sh
sudo apt install build-essential
```

**WARNING** To build you must have gcc version 10 or greater due to the C++20
features I use. Ubuntu 20.04 ships gcc 9. They do not backport newer gcc versions,
so a ppa is necessary.
```sh
sudo add-apt-repository ppa:ubuntu-toolchain-r/test
sudo apt update
sudo apt install g++-10
```
You then need to set an environment variable to select g++-10 over the default g++,
which points to g++-9
```sh
CXX=g++-10 cargo build --release
```

Other distributions will vary.

#### Proprietary vendor libraries (time tagger control only)

Download the [vendor libraries][q] to copy over in a later step.

Note: Because these libraries cannot be freely distributed by me, the entire installation
process is somewhat complicated. Sorry. If you are not doing time tagger control at all
(or even just on a client-only computer), you don't need to install these
libraries; replace everything below with the one-liner
`cargo install --git https://git.sr.ht/~rwp/tagger <crate>` for each crate you want to (re)install.

#### capnproto (optional; you are probably not doing this)

Install [capnp][p] if using your own non-Rust (e.g. Python) code to use the capnp APIs,
or if you are modifying/extending the schema and need to regenerate Rust code. Generated
code is checked into the repository, so for most purposes it is not necessary to install
the capnp tool.

### Installation

#### Clone the repository

These instructions are for Linux. In Windows, make directories and copy over files as indicated.

Open a shell. Using [git][g], clone the repository locally and change directory into it

```sh
cd ~
mkdir code
cd code
git clone https://git.sr.ht/~rwp/tagger
cd tagger
```

The repository is now located in `/home/<user>/code/tagger`. All further commands are assumed to
be in this directory (or wherever else you put it).

#### Copy over vendor libraries

If using for time tagger control, download and copy over [proprietary vendor libraries][q]

```sh
mkdir -p lib
cp /path/to/CTimeTagLib.lib lib  # Windows library
cp /path/to/libtimetag64.so lib  # Linux library
```

In Windows, the library is compiled into the binary (convenient but means you cannot redistribute it),
while on Linux you may need to copy the library to a system lib folder as well:

```sh
sudo cp /path/to/libtimetag64.so /usr/lib
```

#### Compile and install the crates you want

Pick which crates you want to install:

```sh
cargo install --path ./<crate>
cargo install --path ./tagstream # for example
cargo install --path ./tagview   # etc...
```

Installed crates will then be available in your shell's `PATH`, e.g. just run `tagview`
on Linux or `tagview.exe` on Windows.

#### Update/uninstall

If you need to update, pull the changes via git and then reinstall everything

```sh
cd ~/code/tagger
git pull
cargo install --path ./<crate>
cargo install --path ./tagsave # for example
```
verify the new version with
```sh
tagsave --version
```

To uninstall, simply

```sh
cargo uninstall <crate>
```

## Building a release

Tarball releases containing all binaries can be produced using [cargo xtask][xt],
which automates most of the build, tarball, compression, and checksum process.

Checklist for bumping vX.Y.Z to vU.V.W:
1. `git pull` to ensure you have the latest content
2. `git branch` to check you are on `master`
3. Run test suite
4. Bump the version number of all crates and commit this
5. `rg X.Y.Z` to verify all versions bumped
6. `git tag -a vU.V.W` to tag this commit, then write release notes
7. use `git shortlog vX.Y.Z..HEAD` to generate changelist for release notes
8. `git push --follow-tags` to push new commit(s) and tag to remote
9. `cargo xtask dist` to build release tarballs
10. For other architectures, `git pull`, `cargo xtask dist`, concat SHA256 files
11. Upload all release tarballs and SHA256 to tag using web interface

This project uses vendor libraries that cannot be redistributed. They are compiled into
Windows builds of `tagstream`, so `tagstream.exe` is not redistributable. The `cargo xtask`
script will automatically make both a complete, nonredistributable release suitable for internal
use, and a redistributable version without `tagstream.exe` that can be uploaded to the public repository. The Linux binaries do not contain vendor code and so can be distributed freely.

[xt]: https://github.com/matklad/cargo-xtask/

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
Time tag analysis tools. Version 1.0.0, Commit abcd123. https://git.sr.ht/~rwp/tagger
```

Additionally, the tools are all versioned, which is tracked in git with a tag,
which associates something like "v1.0.0" with a specific commit. This information
is also compiled into the programs so, for example, `tcat --version` will report
the version as well.

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