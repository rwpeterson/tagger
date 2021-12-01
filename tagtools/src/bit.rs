//! Bitmask tools for working with patterns of channels

use bit_iter::BitIter;

/// Convert channels into a bitmask
pub fn chans_to_mask(chs: &[u8]) -> u16 {
    let mut m = 0;
    for ch in chs {
        m |= 1 << (ch - 1);
    }
    return m;
}

/// Returns a single channel if the mask has only one channel
pub fn mask_to_single(m: u16) -> Option<u8> {
    match m.count_ones() {
        1 => {
            let mut v = mask_to_chans(m).into_iter();
            Some(v.next().unwrap())
        },
        _ => None,
    }
}

/// Returns a pair of channels if the mask has only two channels
pub fn mask_to_pair(m: u16) -> Option<(u8, u8)> {
    match m.count_ones() {
        2 => {
            let mut v = mask_to_chans(m).into_iter();
            Some((v.next().unwrap(), v.next().unwrap()))
        }
        _ => None,
    }
}

/// Returns all channels in mask
pub fn mask_to_chans(m: u16) -> Vec<u8> {
    let mut chs = Vec::new();
    for b in BitIter::from(m) {
        // Channels are 1-indexed, bits are 0-indexed
        chs.push(1 + b as u8);
    }
    return chs;
}

/// Bitwise set/clear/toggle/check/change operations for u16 and u32

use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign};
use num_traits::{FromPrimitive, PrimInt, Unsigned};

pub trait BitOps:
    PrimInt
    + BitAndAssign
    + BitOrAssign
    + BitXorAssign
    + FromPrimitive
    + Unsigned
{
    fn set(&mut self, b: usize);
    fn clear(&mut self, b: usize);
    fn toggle(&mut self, b: usize);
    fn change(&mut self, b: usize, x: bool);
    fn check(self, b: usize) -> bool;
}

impl BitOps for u8 {
    #[inline]
    fn set(&mut self, b: usize) {
        *self |= 1 << b;
    }

    #[inline]
    fn clear(&mut self, b: usize) {
        *self &= !(1 << b);
    }

    #[inline]
    fn toggle(&mut self, b: usize) {
        *self ^= 1 << b;
    }

    #[inline]
    fn change(&mut self, b: usize, x: bool) {
        *self = (*self & !(1 << b)) | ((x as u8) << b);
    }

    #[inline]
    fn check(self, b: usize) -> bool {
        return self >> b & 1 == 1;
    }
}

impl BitOps for u16 {
    #[inline]
    fn set(&mut self, b: usize) {
        *self |= 1 << b;
    }

    #[inline]
    fn clear(&mut self, b: usize) {
        *self &= !(1 << b);
    }

    #[inline]
    fn toggle(&mut self, b: usize) {
        *self ^= 1 << b;
    }

    #[inline]
    fn change(&mut self, b: usize, x: bool) {
        *self = (*self & !(1 << b)) | ((x as u16) << b);
    }

    #[inline]
    fn check(self, b: usize) -> bool {
        return self >> b & 1 == 1;
    }
}

impl BitOps for u32 {
    #[inline]
    fn set(&mut self, b: usize) {
        *self |= 1 << b;
    }

    #[inline]
    fn clear(&mut self, b: usize) {
        *self &= !(1 << b);
    }

    #[inline]
    fn toggle(&mut self, b: usize) {
        *self ^= 1 << b;
    }

    #[inline]
    fn change(&mut self, b: usize, x: bool) {
        *self = (*self & !(1 << b)) | ((x as u32) << b);
    }

    #[inline]
    fn check(self, b: usize) -> bool {
        return self >> b & 1 == 1;
    }
}

impl BitOps for u64 {
    #[inline]
    fn set(&mut self, b: usize) {
        *self |= 1 << b;
    }

    #[inline]
    fn clear(&mut self, b: usize) {
        *self &= !(1 << b);
    }

    #[inline]
    fn toggle(&mut self, b: usize) {
        *self ^= 1 << b;
    }

    #[inline]
    fn change(&mut self, b: usize, x: bool) {
        *self = (*self & !(1 << b)) | ((x as u64) << b);
    }

    #[inline]
    fn check(self, b: usize) -> bool {
        return self >> b & 1 == 1;
    }
}

impl BitOps for u128 {
    #[inline]
    fn set(&mut self, b: usize) {
        *self |= 1 << b;
    }

    #[inline]
    fn clear(&mut self, b: usize) {
        *self &= !(1 << b);
    }

    #[inline]
    fn toggle(&mut self, b: usize) {
        *self ^= 1 << b;
    }

    #[inline]
    fn change(&mut self, b: usize, x: bool) {
        *self = (*self & !(1 << b)) | ((x as u128) << b);
    }

    #[inline]
    fn check(self, b: usize) -> bool {
        return self >> b & 1 == 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_masks() {
        assert_eq!(0b01, chans_to_mask(&[1]));
        assert_eq!(0b10, chans_to_mask(&[2]));
        assert_eq!(0b11, chans_to_mask(&[1, 2]));
        assert_eq!(0x8000, chans_to_mask(&[16]));
    }

    #[test]
    fn bijective_channel_masks() {
        // Exhaustively check all u16s
        for pat in u16::MIN..=u16::MAX {
            let chs = mask_to_chans(pat);
            assert!(!chs.contains(&0));
            let pat2 = chans_to_mask(&chs);
            assert_eq!(pat, pat2);
            match pat.count_ones() {
                1 => {
                    assert_eq!(Some(chs[0]), mask_to_single(pat));
                    assert_eq!(None, mask_to_pair(pat));
                },
                2 => {
                    assert_eq!(None, mask_to_single(pat));
                    assert_eq!(Some((chs[0], chs[1])), mask_to_pair(pat));
                },
                _ => {
                    assert_eq!(None, mask_to_single(pat));
                    assert_eq!(None, mask_to_pair(pat));
                },
            }
        }
    }

    #[test]
    fn bit_ops() {
        // Exhaustively check all u8's
        for i in u8::MIN..=u8::MAX {
            for b in BitIter::from(u8::MAX) {
                let mut x = i;
                let i_set = i | 1 << b;
                let i_clr = i & !(1 << b);

                assert_eq!(i.check(b), i >> b & 1 == 1);
                x.set(b);
                assert_eq!(x, i_set);
                x.clear(b);
                assert_eq!(x, i_clr);
                x.toggle(b);
                assert_eq!(x, i_set);
                x.toggle(b);
                assert_eq!(x, i_clr);
            }
        }
        // Exhaustively check all u16's
        for i in u16::MIN..=u16::MAX {
            for b in BitIter::from(u16::MAX) {
                let mut x = i;
                let i_set = i | 1 << b;
                let i_clr = i & !(1 << b);

                assert_eq!(i.check(b), i >> b & 1 == 1);
                x.set(b);
                assert_eq!(x, i_set);
                x.clear(b);
                assert_eq!(x, i_clr);
                x.toggle(b);
                assert_eq!(x, i_set);
                x.toggle(b);
                assert_eq!(x, i_clr);
            }
        }
        // Check some interesting u32's
        for &i in [
            u32::MIN,
            1u32,
            1337u32,
            u32::MAX
        ].iter() {
            for b in BitIter::from(u32::MAX) {
                let mut x = i;
                let i_set = i | 1 << b;
                let i_clr = i & !(1 << b);

                assert_eq!(i.check(b), i >> b & 1 == 1);
                x.set(b);
                assert_eq!(x, i_set);
                x.clear(b);
                assert_eq!(x, i_clr);
                x.toggle(b);
                assert_eq!(x, i_set);
                x.toggle(b);
                assert_eq!(x, i_clr);
            }
        }
        // Check some interesting u64's
        for &i in [
            u64::MIN,
            1u64,
            1337u64,
            u64::MAX
        ].iter() {
            for b in BitIter::from(u64::MAX) {
                let mut x = i;
                let i_set = i | 1 << b;
                let i_clr = i & !(1 << b);

                assert_eq!(i.check(b), i >> b & 1 == 1);
                x.set(b);
                assert_eq!(x, i_set);
                x.clear(b);
                assert_eq!(x, i_clr);
                x.toggle(b);
                assert_eq!(x, i_set);
                x.toggle(b);
                assert_eq!(x, i_clr);
            }
        }
        // Check some interesting u128's
        for &i in [
            u128::MIN,
            1u128,
            1337u128,
            u128::MAX
        ].iter() {
            for b in BitIter::from(u128::MAX) {
                let mut x = i;
                let i_set = i | 1 << b;
                let i_clr = i & !(1 << b);

                assert_eq!(i.check(b), i >> b & 1 == 1);
                x.set(b);
                assert_eq!(x, i_set);
                x.clear(b);
                assert_eq!(x, i_clr);
                x.toggle(b);
                assert_eq!(x, i_set);
                x.toggle(b);
                assert_eq!(x, i_clr);
            }
        }
    }
}
