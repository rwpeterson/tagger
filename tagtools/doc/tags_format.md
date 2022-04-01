# The `.tags.zst` format

We store tags on-disk using a schema implemented by [Cap'n Proto][c]
(the "`.tags`" part), additionally compressed with [Zstandard][z]
(the "`.zst`" part). This document describes the format in detail,
gives some rationale for its design choices, and presents some
benchmarks against other candidate formats.

## What is the `tags.zst` format?

These files consistently use the `.tags.zst` extension, and reference
implementations of tools to work with them are in the `tagtools`
crate. The `tags.capnp` file that defines the schema is in the
`tagger_capnp` crate. It will (hopefully) never need to change,
so I can copy it here as well:

```capnproto
@0xd932ef88b339497e;
struct Tags @0xb1642a9902d01394 {  # 0 bytes, 1 ptrs
  tags @0 :List(List(Tag));  # ptr[0]
  struct Tag @0x8995b3a3aece585b {  # 16 bytes, 0 ptrs
    time @0 :Int64;  # bits[0, 64)
    channel @1 :UInt64;  # bits[64, 128)
  }
}
```

The unique IDs and byte layout comments of the structs are automatically
produced by the `capnpc` compiler, but are retained in the source to make
it easier for someone with no knowledge of the format to hand-write a
parser if need be. Because a `u8` channel might be somewhat limiting,
in this schema I use a `u64` instead. As we will see, this has no overhead
due to alignment, and future-proofs the format. Below, I still refer to it
as a `u8` since this more closely matches the vendor's `unsigned char` type,
and in practice deserialized channels are cast to `u8` anyway.

## How do I use data saved in the `tags.zst` format?

In addition to making our software easy to use, scientific work should
be reproducible by others. This means that others should be able to use
our software, as well as our data (even if they don't want to use our
software to do it). Our format can be accessed in several ways, outlined
below.

### The easy way

The simplest way is to use the provided `tcat` program, which decodes
the data and outputs tab-separated values that are fully equivalent to
how data has been stored before. This can also be used as part of an
analysis script, so the decompressed tags don't need to take up extra
space on your filesystem. The `txt2tags` program does the reverse,
converting old data to the new format.

```sh
tcat mydata.tags.zst > mydata.txt
```

### "I don't want to use your program"

You can generate code to work with the format with any supported language:
all you need is the `tags.capnp` file and the `capnp` compiler to generate
the appropriate code for your language. We use the default unpacked
serialization, and compress this further with Zstandard. You can use
your preferred language's bindings to Zstandard, or decompress tags
yourself with the `zstd` system utility before deserializing.

You can even use the `capnp` program at the command line to decode any
message to a human-readable format:

```sh
zstd -d mydata.tags.zst # decompresses to mydata.tags
capnp decode tags.capnp Tags < mydata.tags > mydata.txt
```

`capnp` supports many languages, and is packaged in all major distros.

### "I don't want to use anyone else's program either"

If your language does not have Cap'n Proto support, `capnc` can tell you
the bit layout of the message format (see above), allowing you to write
a parser yourself. For our schema, the format is easy to see in a hex
editor: all data is organized in 64-bit words. At the beginning there is
a message frame word describing how long the message is, a root struct
pointer word that points to subsequent data and pointer locations, a
list pointer word that describes our list of tags, and a tag struct
pointer word that describes each element of the list. Following this are
two-word list elements of tags.

## Why was this specific format chosen?

There are many different ways to store data, with different tradeoffs.
One of the simplest is CSV, or comma-separated values. Historically
we have used CSV (really, tab-separated so TSV) as a format for tags,
as it's human-readable, straightforward to understand, and has tools
available in essentially every computer language.

However, CSV as a format is extremely inefficient for numerical data,
as the numbers are represented in decimal and encoded as ASCII text.
With increasingly large datasets, these files take up increasing
amounts of disk space, and are unwieldy to analyze as well.

## A binary format

Using a binary format is more efficient from two points of view. One is
that a more space-efficient encoding can be chosen, using less on-disk
space than something like a CSV. The other is that binary formats are
significantly less computationally-intensive to parse and compress,
meaning there is much less overhead at a constant file size. There is
little limitation on how to choose such a format. Here are some realistic
choices:

### DIY format

The simplest solution is to define your own simple binary format, with
certain offsets and lengths of bits being interpreted as integers, for
example. For tags, we have an `i64` timestamp and a `u8` channel, so 64 +
8 = 72 bits = 9 bytes per tag for a 1-to-1 representation.  There is no
hierarchical structure to the tags, so we can simply repeat this 9 byte
record for as many tags as we have. Ideally, the format would start with
a header or "magic number" to help identity the file.

This approach has the disadvantages of requiring custom parsers to be
written by hand in every language using this format, likely introducing
bugs. It also leaves some efficiency on the table, as the i64s in our
case are nonnegative and usually small, so a variable-length encoding
is an excellent choice to save more space. Implementing this extension
by hand (and without errors!) is an unreasonable expectation for anyone
wanting to use tags with a different programming language. Furthermore,
any additional formats for space-intensive data other than tags start
from square one and also need to be written from scratch.

### Your favorite language's serialization scheme

Almost all languages offer some way to serialize objects to store on
disk. Almost all of them are a poor choice as an on-disk file format.
As an example, Python provides pickle for this task. As a durable
data format, pickle turns out to be a bad idea, and is an instructive
example whose criticisms apply to most other schemes.  First, it does
not have a stable on-disk format: it is at least forward-compatible,
but older versions of Python cannot read pickles created from newer
versions. Second, it is specific to Python. Yes, we can all write Python,
but it's not the right tool for every job. Third, it's fundamentally
insecure in that it executes arbitrary code in the pickle object. Even
though it is architecture-independent, for this reason it's bad as a
data interchange format, as opening an untrusted pickle is dangerous.

### "Scientific" data serialization formats

There are a few serialization formats for scientific applications, such
as HDF. HDF seems ideally-suited to our use case, but in practice has
[serious issues][b]. Most importantly, while it has a specification,
there is only the single reference implementation in C. Complexity in
the spec leads to bugs, deviation from the spec in the implementaiton,
and (as many anecdotes attest) data corruption. Bad news! Its speed
of writing and reading data is also relatively slow, and we need to save
up to 10 million tags per second.

The lesson here is that scientific software is often not that great,
has a small userbase who may have little knowledge of programming or
software engineering practices, and may only have good performance in a
few domains (large datasets, efficient computation) at the expense
of others (easy API, stability, security). We should not be too quick to
cargo-cult bigtech tools and software engineering practices in response,
but identifying where they have solved useful general-purpose problems
can help save us time and effort.

Taking a step back, we can see that HDF provides a "solution" to some
problems that don't need to be solved. It provides a filesystem-like
hierarchical structure--just use your filesystem instead and zip it up! We
are concerned in this application with a single, linear stream of a single
type of data that arrives ordered in time, so even a single-file database
is overkill.

[b]: https://cyrille.rossant.net/moving-away-hdf5/

### Schema-based data serialization

For high-performance applications requiring a well-defined message format,
several binary encodings exist. Of the modern open-source frameworks,
realistic options are [Protocol Buffers][p], [Cap'n Proto][c],
[flatbuffers][f], and [SBE][s].  They all use an interface description
language to define the data to be serialized. This description is used
by a code generator program (typically written in C++) which outputs a
library in any supported language that allows you to read from or write
to the protocol you defined. These formats often have efficient encoding,
have a schema that allows you to upgrade the protocol while maintaining
forwards and backwards compatibility, and are proven in high-performance,
high-reliability applications in the modern Web.

Recent preference seems to point towards the "zero-copy" formats,
over the older Protocol Buffers ("Protobufs"). Google's Protobufs have
an in-memory representation of the message that is different than the
on-the-wire representation, so the message must be parsed on each end,
with a small performance penalty.  As the parsers must also be generated
for each protocol you specify, the generated code can be quite large.

Of the zero-copy formats, Cap'n Proto is the oldest. The author
wrote Protobuf v2 at Google before leaving to design his own format
at a startup he founded; the company was acquired by Cloudflare, who
maintains Cap'n Proto and uses it internally. Flatbuffers are a newer
format from Google, similar to Cap'n Proto in design, with an original
application to game programming. Finally, SBE is a financial industry
standard with an emphasis on raw speed and reliability, e.g. in
high-frequency trading.

The upside of these formats is that a schema file both documents and
defines the format for you, generates code to interface with it (including
validating the data), and allows others to work with your format without
sharing the same language or worrying about bugs in hand-written parsers.

The downside of these formats is that language support is not
universal. Notably, SBE doesn't even support Python!  Specialist languages
for scientific computing are lagging in adoption for all of them. Aside
from performance, we need to consider whether the tools are available
in relevant languages:

| Lang        | Protobuf | Cap'n Proto | Flatbuffers |
| ----        | ---      | ---         | ---         |
| C++         | yes      | yes         | yes         |
| Rust        | yes      | yes         | yes         |
| Java/.NET   | yes      | yes         | yes         |
| Python      | yes      | yes         | yes         |
| Julia       | yes      |             | blocked     |
| Matlab      | hacks    |             |             |
| Mathematica |          |             |             |
| R           | yes      |             |             |

[p]: https://developers.google.com/protocol-buffers/
[f]: https://google.github.io/flatbuffers/
[s]: https://real-logic.github.io/simple-binary-encoding/

### RPC for free

An additional benefit of modern network serialization formats is that most
have a corresponding remote procedure call (RPC) functionality built on
top of them. A commonplace example today is JSON and JSON-RPC. For the
formats under consideration by us, both Protobuf and flatbuffers support
gRPC, and Cap'n Proto has an integrated RPC framework. By choosing
a serialization format based on our most demanding task (storing and
working with fast streams of time tags), we can become familiar with it,
and use that knowledge for other tasks, like making client-server APIs
for lab equipment, and indeed entire experiments.

## Compression

Even binary formats benefit from compression. New compression algorithms
like Zstandard have a very high compression ratio, while still being
very efficient in encoding/decoding. As the tag stream does not saturate
the write speed of a hard drive, we can apply compression essentially for
free, compared to the speed of writing to disk.

## Benchmarks

We start with 500k tags, stored as tab-separated values with Windows
line endings ("\\r\\n"). Note that Unix line endings ("\\n") are one byte
smaller. Serialized tags are written to an uncompressed file, and
the compression time and performance is measured separately with the
`zstd` utility rather than library bindings in the language used for
serialization (Rust). The time to compress is measured, e.g.:

    time zstd -c tags_500k.tagscp > /dev/null

Files compressed in this manner may be very slightly different than those
using the libzstd bindings with the same options, but are essentially equal
in size and are fully interoperable, deserializing to identical content.

| MB    | Ratio | Format                        | Comp. time (ms) |
| ----- | :---: | :---------------              | ---:            |
| 8.276 | 1     | CSV (Windows)                 |                 |
| 8.000 | 1.04  | Cap'n Proto (unpacked)        |                 |
| 8.000 | 1.04  | flatbuffer                    |                 |
| 7.776 | 1.06  | CSV (Unix)                    |                 |
| 5.500 | 1.50  | Protobuf                      |                 |
| 4.500 | 1.84  | Custom binary                 |                 |
| 3.992 | 2.07  | Cap'n Proto                   |                 |
| 3.611 | 2.29  | Cap'n Proto + zstd --fast=5   |   26            |
| 2.829 | 2.93  | Cap'n Proto + LZ4             |   28            |
| 2.764 | 2.99  | CSV (Windows) + zstd          |  153            |
| 2.739 | 3.02  | Cap'n Proto + zstd -1         |   48            |
| 2.738 | 3.02  | CSV (Unix) + zstd             |  130            |
| 2.338 | 3.54  | Custom binary + zstd          |   87            |
| 2.004 | 4.13  | Protobuf + zstd               |   84            |
| 1.835 | 4.51  | Cap'n Proto + zstd            |   73            |
| 1.788 | 4.63  | Cap'n Proto (unpacked) + zstd |   97            |
| 1.787 | 4.63  | flatbuffer + zstd             |   53*           |
| 1.780 | 4.65  | Cap'n Proto + zstd -19        | 2080            |
| 1.728 | 4.79  | CSV (Unix) + zstd -19         | 7640            |

\* Different computer with faster single-core performance

A first observation is that the unpacked Cap'n Proto format is quite
large.  This is because it's 8-byte aligned, and our tag is 9 bytes: thus
there are 7 extra bytes of zeros per tag in this representation. My custom binary
format, which is unaligned, uses 8 bytes for a `i64` timestamp and 1 byte for a
`u8` channel per tag. Packing improves on this: it uses variable-length
encoding to avoid repeated zeroes due to alignment of small types like our
`u8` channel, as well as small values in large types, like our timestamps
which often have many leading zeros. Protobuf v3 also uses packed values by
default for `repeated` entries, although both the packing and subsequent
compression are less efficient than Cap'n Proto. As for compression, LZ4,
a format focused on fast (de)compression, is a good baseline. Zstandard,
which emphasizes an optimal tradeoff between both speed and compression
ratio, has excellent performance at its default compression level of
3. At its highest level, 19, there is very little additional compression
(and practically, it is far too slow for our application). Conversely,
at the faster level 1 and fastest level (-5), we are leaving space
savings on the table.

The zero-copy formats (Cap'n Proto and flatbuffers) combined with Zstandard
compression are the best performers, considering both compression ratio and
the speed of the compression step. As a tiebreaker, we note that flatbuffers
support only 2 GiB messages, as they are a 32-bit format. We can easily
acquire more than 2 GiB of tags in a short time, so this is unacceptable.
Cap'n Proto is a 64-bit format, and while there are a few nuances, in
principle messages can exceed 1 EiB in size.

For users of an unsupported language, the wire format of Cap'n Proto
messages can be determined by:

    capnp compile -ocapnp myschema.capnp

which returns an annotated version of the schema. This allows you to write
a simple parser to work with the format in a language lacking support.
Zstandard has bindings for essentially every language (including
Mathematica, etc.), and one can always use the `zstd` system utility to
decompress separately. For our purposes, we store this annotated version
of the schema, so anyone with a copy of our source code repository can
parse the file by hand if need be, without even needing to run the `capnp`
utility.

### Large file performance

These tests were done on a 605 MB CSV file of 35 million tags.  A Rust
utility parses the CSV file, loads it into memory as a `Vec<Tag>`,
serializes it with either Cap'n Proto, Protobuf, or flatbuffer,
stream-compresses it with zstd, and finally writes it to a file. Test was
on a T14 Gen 1 with Ryzen 7 Pro 4750U and NVME SSD. `Tag` is defined by
`struct Tag { time: i64, channel: u8 }` and the schema files specify
the closest equivalent to `Vec<Tag>`.

    time ./tsv_capnp tags_35M.tsv > tags_35M.cptags # 13.097 sec
    time ./tsv_proto tags_35M.tsv > tags_35M.pbtags # 27.866 sec
    time ./tsv_flatb tags_35M.tsv > tags_35M.fbtags # 12.525 sec

The benchmark was ran twice (repeated in the order listed) to ensure
there was no bias due to caching.

|        Operation          | Time (s) | Size (MB) | Ratio |
| :------------------------ | -------- | --------- | ----- |
|        Original CSV       |    -     |   605     |   1   |
| CSV ->    Protobuf + zstd | 27.866   |   142     | 4.26  | 
| CSV -> Cap'n Proto + zstd | 13.097   |   128     | 4.73  |
| CSV ->  flatbuffer + zstd | 12.525   |   128     | 4.73  |

The 2x speed of the zero-copy formats  over Protobuf is notable (and
independent of a packed or unpacked format). It is likely even better,
as both programs have the same costly CSV parsing as a constant factor.
To be fair, one should be wary of my implementation. Perhaps the Protobuf
code could be improved, and more detailed profiling could determine if
indeed its extra parsing step is the bottleneck here. Supposedly, in
Protobuf it's more performant to use parallel arrays instead of an array
of structs, but this is an ergonomic drawback of Protobufs and perhaps
bad for a format where the tag structs are received and collected in
time-order. Likewise, the 10% larger file size with Protobuf is consistent
with the first test dataset above, and unlikely to change.  This is
a substantial amount of space savings with Cap'n Proto/flatbuffers,
and hard to say no to.

## Packed or unpacked?

    time ./tsv_unpacked tags_35M | wc -c # real  9.213s user  8.735s sys  0.812s
    time ./tsv          tags_35M | wc -c # real 10.003s user  9.066s sys  1.148s
    time ./tsv2tsv      tags_35M | wc -c # real 13.585s user 11.175s sys  5.688s
    time ./tsv2tsv.zst  tags_35M | wc -c # real 17.776s user 16.631s sys  1.828s
 
It seems like there is a few-percent speed improvement using the unpacked
encoding, and allowing zstd to do the job of compression.  There was
also a few-percent  decrease in size with the compressed unpacked
format above. This is likely because the packing shifts the alignment
of tags back and forth, making it a little more difficult for zstd to
find compressible information. Plus, it's easier to write a parser for
the unpacked format since you don't need to implement that packing algorithm.
As a comparison, we parse a CSV, read into memory, and write to CSV again. Including
the same level of compression, it takes nearly twice as long.

[c]: https://capnproto.org
[z]: https://facebook.github.io/zstd/
