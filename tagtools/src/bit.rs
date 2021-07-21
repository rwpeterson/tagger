use bit_iter::BitIter;

pub fn chans_to_mask(chs: &[u8]) -> u16 {
    let mut m = 0;
    for ch in chs {
        m |= 1 << (ch - 1);
    }
    return m;
}

/// Returns a single channel if the mask is one channel
pub fn mask_to_single(m: u16) -> Option<u8> {
    match m.count_ones() {
        1 => {
            let mut v = mask_to_chans(m).into_iter();
            Some(v.next().unwrap())
        },
        _ => None,
    }
}

/// Returns a pair of channels if the mask is two channels
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks() {
        assert_eq!(0b01, chans_to_mask(&[1]));
        assert_eq!(0b10, chans_to_mask(&[2]));
        assert_eq!(0b11, chans_to_mask(&[1, 2]));
        assert_eq!(0x8000, chans_to_mask(&[16]));
    }

    #[test]
    fn bijective() {
        // Exhaustively check all u16s
        let pats = (u16::MIN..=u16::MAX).collect::<Vec<_>>();

        for pat in pats {
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
}