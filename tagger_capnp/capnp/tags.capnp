@0xd932ef88b339497e;

# For current devices, a u8 suffices for the channel. With 128-channel
# time taggers in development, and the choice of vendors to 1-index
# channels in their APIs, a u8 is essentially already at its limit.
# Because of this, we instead use a u64. Due to the 8-byte alignment in
# Cap'n Proto, there is no cost to this versus any smaller int type. It
# also enables applications like storing up to 64-bit pattern mask
# events, and virtually guarantees that the tags format can be frozen.
# Keeping consumer code as u8 initially is fine, as the u8 -> u64 cast
# in serialization always succeeds, and the u64 -> u8 cast in
# deserialization can be upgraded when the need arises.

struct Tags @0xb1642a9902d01394 {  # 0 bytes, 1 ptrs
  tags @0 :List(List(Tag));  # ptr[0]
  struct Tag @0x8995b3a3aece585b {  # 16 bytes, 0 ptrs
    time @0 :Int64;  # bits[0, 64)
    channel @1 :UInt64;  # bits[64, 128)
  }
}