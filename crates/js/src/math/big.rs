//! The little multiprecision arithmetic the correctly rounded `sin`/`cos`
//! fall back on: unsigned integers of 64-bit limbs, used as fixed-point
//! numbers with a given count of fraction bits.

use std::cmp::Ordering;

/// An unsigned integer, least significant limb first, without leading
/// (high) zero limbs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Big(Vec<u64>);

impl Big {
    pub(super) fn zero() -> Big {
        Big(Vec::new())
    }

    pub(super) fn from_u64(n: u64) -> Big {
        Big(vec![n]).normalized()
    }

    /// `2^n`.
    pub(super) fn pow2(n: usize) -> Big {
        let mut limbs = vec![0; n / 64 + 1];
        limbs[n / 64] = 1 << (n % 64);
        Big(limbs)
    }

    fn normalized(mut self) -> Big {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    pub(super) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of significant bits (0 for zero).
    pub(super) fn bits(&self) -> usize {
        match self.0.last() {
            None => 0,
            Some(top) => self.0.len() * 64 - top.leading_zeros() as usize,
        }
    }

    pub(super) fn bit(&self, i: usize) -> bool {
        self.0
            .get(i / 64)
            .is_some_and(|limb| limb >> (i % 64) & 1 == 1)
    }

    /// The low 64 bits.
    pub(super) fn low_u64(&self) -> u64 {
        self.0.first().copied().unwrap_or(0)
    }

    /// Whether any of the low `n` bits is set.
    pub(super) fn any_below(&self, n: usize) -> bool {
        let whole = (n / 64).min(self.0.len());
        if self.0[..whole].iter().any(|&limb| limb != 0) {
            return true;
        }
        !n.is_multiple_of(64)
            && self
                .0
                .get(n / 64)
                .is_some_and(|limb| limb & ((1 << (n % 64)) - 1) != 0)
    }

    pub(super) fn shl(&self, n: usize) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        let (limbs, bits) = (n / 64, n % 64);
        let mut out = vec![0; limbs];
        if bits == 0 {
            out.extend_from_slice(&self.0);
        } else {
            let mut carry = 0;
            for &limb in &self.0 {
                out.push(limb << bits | carry);
                carry = limb >> (64 - bits);
            }
            out.push(carry);
        }
        Big(out).normalized()
    }

    pub(super) fn shr(&self, n: usize) -> Big {
        let (limbs, bits) = (n / 64, n % 64);
        if limbs >= self.0.len() {
            return Big::zero();
        }
        let src = &self.0[limbs..];
        let out = if bits == 0 {
            src.to_vec()
        } else {
            (0..src.len())
                .map(|i| src[i] >> bits | src.get(i + 1).map_or(0, |next| next << (64 - bits)))
                .collect()
        };
        Big(out).normalized()
    }

    pub(super) fn add(&self, other: &Big) -> Big {
        let (long, short) = if self.0.len() >= other.0.len() {
            (self, other)
        } else {
            (other, self)
        };
        let mut out = Vec::with_capacity(long.0.len() + 1);
        let mut carry = false;
        for (i, &a) in long.0.iter().enumerate() {
            let (sum, c1) = a.overflowing_add(short.0.get(i).copied().unwrap_or(0));
            let (sum, c2) = sum.overflowing_add(u64::from(carry));
            out.push(sum);
            carry = c1 || c2;
        }
        out.push(u64::from(carry));
        Big(out).normalized()
    }

    /// `self - other`; `other` must not be greater.
    pub(super) fn sub(&self, other: &Big) -> Big {
        debug_assert!(*self >= *other, "negative difference");
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow = false;
        for (i, &a) in self.0.iter().enumerate() {
            let (diff, b1) = a.overflowing_sub(other.0.get(i).copied().unwrap_or(0));
            let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
            out.push(diff);
            borrow = b1 || b2;
        }
        Big(out).normalized()
    }

    pub(super) fn mul(&self, other: &Big) -> Big {
        if self.is_zero() || other.is_zero() {
            return Big::zero();
        }
        let mut out = vec![0u64; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.0.iter().enumerate() {
                let t = u128::from(a) * u128::from(b) + u128::from(out[i + j]) + carry;
                out[i + j] = t as u64;
                carry = t >> 64;
            }
            out[i + other.0.len()] = carry as u64;
        }
        Big(out).normalized()
    }

    #[cfg(test)]
    pub(super) fn mul_u64(&self, n: u64) -> Big {
        self.mul(&Big::from_u64(n))
    }

    /// `floor(self / n)`.
    pub(super) fn div_u64(&self, n: u64) -> Big {
        let mut out = vec![0; self.0.len()];
        let mut rem = 0u128;
        for i in (0..self.0.len()).rev() {
            let cur = rem << 64 | u128::from(self.0[i]);
            out[i] = (cur / u128::from(n)) as u64;
            rem = cur % u128::from(n);
        }
        Big(out).normalized()
    }

    /// `(floor(self / d), self mod d)` by shift and subtract; `d` not zero.
    pub(super) fn div_rem(&self, d: &Big) -> (Big, Big) {
        assert!(!d.is_zero(), "division by zero");
        if self < d {
            return (Big::zero(), self.clone());
        }
        let shift = self.bits() - d.bits();
        let mut rem = self.clone();
        let mut quotient = vec![0u64; shift / 64 + 1];
        for s in (0..=shift).rev() {
            let shifted = d.shl(s);
            if rem >= shifted {
                rem = rem.sub(&shifted);
                quotient[s / 64] |= 1 << (s % 64);
            }
        }
        (Big(quotient).normalized(), rem)
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, other: &Big) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Big {
    fn cmp(&self, other: &Big) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// `π · 2^frac`, truncated: below the exact value by less than 1.
///
/// Machin's formula, π = 16 atan(1/5) - 4 atan(1/239), with 64 guard bits;
/// each series term is truncated once (at most 1 below), and there are
/// fewer than `frac` of them, so the sum is within 2^64 below and the
/// shift leaves less than 1 of error.
pub(super) fn pi(frac: usize) -> Big {
    let bits = frac + 64;
    // atan(1/n) · 2^bits as (positive terms, negative terms).
    let atan_inv = |n: u64| {
        let mut power = Big::pow2(bits).div_u64(n);
        let (mut plus, mut minus) = (power.clone(), Big::zero());
        for k in 1u64.. {
            power = power.div_u64(n * n);
            if power.is_zero() {
                break;
            }
            let term = power.div_u64(2 * k + 1);
            if k % 2 == 1 {
                minus = minus.add(&term);
            } else {
                plus = plus.add(&term);
            }
        }
        (plus, minus)
    };
    let (p5, m5) = atan_inv(5);
    let (p239, m239) = atan_inv(239);
    let plus = p5.shl(4).add(&m239.shl(2));
    let minus = m5.shl(4).add(&p239.shl(2));
    plus.sub(&minus).shr(64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let a = Big::from_u64(u64::MAX).mul(&Big::from_u64(u64::MAX));
        assert_eq!(a.bits(), 128);
        // (2^64 - 1)^2 = 2^64 (2^64 - 2) + 1
        assert_eq!(a.shr(64), Big::from_u64(u64::MAX - 1));
        assert_eq!(a.low_u64(), 1);
        let (q, r) = a.add(&Big::from_u64(5)).div_rem(&Big::from_u64(u64::MAX));
        assert_eq!((q, r), (Big::from_u64(u64::MAX), Big::from_u64(5)));
        assert_eq!(Big::pow2(130).shr(129), Big::from_u64(2));
        assert_eq!(Big::pow2(130).sub(&Big::from_u64(1)).bits(), 130);
        assert!(Big::pow2(130).any_below(131));
        assert!(!Big::pow2(130).any_below(130));
        assert_eq!(Big::from_u64(1000).div_u64(7), Big::from_u64(142));
    }

    #[test]
    fn pi_digits() {
        // π = 3.243F6A8885A308D313198A2E03707344A... in hexadecimal.
        let pi = pi(124);
        assert_eq!(pi.bits(), 126);
        assert_eq!(pi.shr(64).low_u64(), 0x3243_F6A8_885A_308D);
        assert_eq!(pi.low_u64(), 0x3131_98A2_E037_0734);
    }
}
