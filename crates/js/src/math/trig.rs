//! `Math.sin` and `Math.cos`, correctly rounded (the double nearest the
//! exact value), written here from the textbook method (Ziv's strategy):
//!
//! 1. Arguments below 2^20 in magnitude are reduced by the nearest multiple
//!    k of π/2, with π/2 split into three doubles (Cody and Waite) and the
//!    products formed exactly (Dekker), into a double-double r with
//!    |r| ≤ π/4 + ε. The sine or cosine of r comes from its Taylor series,
//!    evaluated in double-double arithmetic (about 2^-100 relative error).
//!    If every value within the error bound rounds to the same double, that
//!    double is the result.
//! 2. Otherwise, and for larger arguments, the reduction and the series are
//!    redone in fixed-point integer arithmetic with π from Machin's formula,
//!    at 128 bits of precision and then doubling, until the error interval
//!    rounds to one double. The sine and cosine of a nonzero double are
//!    irrational, so they never fall on a rounding boundary and the loop
//!    ends.
//!
//! Chromium 147 gives the correctly rounded value on all but 23 `sin` and
//! 10 `cos` results of 13,009 measured arguments, each within 0.013 ULP of
//! a rounding boundary (`docs/framework-gaps.md`); V8's own implementation is
//! not ported, for its licence. Those arguments are a recorded fidelity gap,
//! listed in `tests/fixtures/chrome_trig.json`.

use super::big::{pi, Big};

/// `Math.sin(x)`, correctly rounded.
pub fn sin(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    // |x| < 2^-26: x^3/6 is below half an ULP of x, so x is the nearest
    // double (and keeps the sign of zero).
    if x.abs() < TWO_M26 {
        return x;
    }
    fast(x, false).unwrap_or_else(|| slow(x, false))
}

/// `Math.cos(x)`, correctly rounded.
pub fn cos(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    // |x| < 2^-27: x^2/2 is below half an ULP under 1.
    if x.abs() < TWO_M26 / 2.0 {
        return 1.0;
    }
    fast(x, true).unwrap_or_else(|| slow(x, true))
}

const TWO_M26: f64 = 1.0 / 67_108_864.0;

/// π/2 as the unevaluated sum of three doubles (within 2^-164 of it,
/// relative).
const HALF_PI: [f64; 3] = [
    1.5707963267948966,
    6.123233995736766e-17,
    -1.4973849048591698e-33,
];

/// 1/n! for n = 0..=30 as double-doubles (hi, lo).
const INV_FACT: [(f64, f64); 31] = [
    (1.0, 0.0),
    (1.0, 0.0),
    (0.5, 0.0),
    (0.16666666666666666, 9.25185853854297e-18),
    (0.041666666666666664, 2.3129646346357427e-18),
    (0.008333333333333333, 1.1564823173178714e-19),
    (0.001388888888888889, -5.300543954373577e-20),
    (0.0001984126984126984, 1.7209558293420705e-22),
    (2.48015873015873e-05, 2.1511947866775882e-23),
    (2.7557319223985893e-06, -1.858393274046472e-22),
    (2.755731922398589e-07, 2.3767714622250297e-23),
    (2.505210838544172e-08, -1.448814070935912e-24),
    (2.08767569878681e-09, -1.20734505911326e-25),
    (1.6059043836821613e-10, 1.2585294588752098e-26),
    (1.1470745597729725e-11, 2.0655512752830745e-28),
    (7.647163731819816e-13, 7.03872877733453e-30),
    (4.779477332387385e-14, 4.399205485834081e-31),
    (2.8114572543455206e-15, 1.6508842730861433e-31),
    (1.5619206968586225e-16, 1.1910679660273754e-32),
    (8.22063524662433e-18, 2.2141894119604265e-34),
    (4.110317623312165e-19, 1.4412973378659527e-36),
    (1.9572941063391263e-20, -1.3643503830087908e-36),
    (8.896791392450574e-22, -7.911402614872376e-38),
    (3.868170170630684e-23, -8.843177655482344e-40),
    (1.6117375710961184e-24, -3.6846573564509766e-41),
    (6.446950284384474e-26, -1.9330404233703465e-42),
    (2.4795962632247976e-27, -1.2953730964765229e-43),
    (9.183689863795546e-29, 1.4303150396787322e-45),
    (3.279889237069838e-30, 1.5117542744029879e-46),
    (1.1309962886447716e-31, 1.0498015412959506e-47),
    (3.7699876288159054e-33, 2.5870347832750324e-49),
];

// Error-free transformations and double-double arithmetic (Knuth, Dekker).
// Rust never fuses a multiply and an add, so these are exact as written.

/// `a + b` as `s + e` exactly.
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// `a + b` as `s + e` exactly, when |a| ≥ |b| (or a is zero).
fn fast_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}

/// Veltkamp's split of `a` into two 26-bit halves.
fn split(a: f64) -> (f64, f64) {
    let c = 134_217_729.0 * a;
    let hi = c - (c - a);
    (hi, a - hi)
}

/// `a * b` as `p + e` exactly.
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
}

type Dd = (f64, f64);

fn dd_mul(a: Dd, b: Dd) -> Dd {
    let (p, e) = two_prod(a.0, b.0);
    fast_two_sum(p, e + (a.0 * b.1 + a.1 * b.0))
}

fn dd_add(a: Dd, b: Dd) -> Dd {
    let (s, e) = two_sum(a.0, b.0);
    let (t, f) = two_sum(a.1, b.1);
    let (s, e) = fast_two_sum(s, e + t);
    fast_two_sum(s, e + f)
}

/// The sine (`cosine` false) or cosine of the double-double `r`,
/// |r| ≤ 0.8: its Taylor series to the r^29 or r^30 term (the rest is
/// below 2^-111), by Horner's rule in r².
fn series_dd(r: Dd, cosine: bool) -> Dd {
    let z = dd_mul(r, r);
    let (first, terms) = if cosine { (0, 16) } else { (1, 15) };
    let coefficient = |j: usize| {
        let (hi, lo) = INV_FACT[first + 2 * j];
        if j % 2 == 1 {
            (-hi, -lo)
        } else {
            (hi, lo)
        }
    };
    let mut acc = coefficient(terms - 1);
    for j in (0..terms - 1).rev() {
        acc = dd_add(dd_mul(acc, z), coefficient(j));
    }
    if cosine {
        acc
    } else {
        dd_mul(acc, r)
    }
}

/// `hi + lo` rounded, if every real within `err` of it rounds to the same
/// double; `hi` must be a normal number.
fn round_if_sure(hi: f64, lo: f64, err: f64) -> Option<f64> {
    let (h, l) = two_sum(hi, lo);
    // Half the gaps to the neighbouring doubles (exact), less a margin for
    // the rounding of `l ± err`.
    let margin = 1.0 - 1.0 / 1_073_741_824.0;
    let up = (h.next_up() - h) * 0.5 * margin;
    let down = (h - h.next_down()) * 0.5 * margin;
    (h != 0.0 && l + err < up && l - err > -down).then_some(h)
}

/// The double-double path, for |x| < 2^20; `None` when it cannot decide.
fn fast(x: f64, cosine: bool) -> Option<f64> {
    if x.abs() >= 1_048_576.0 {
        return None;
    }
    let k = (x * std::f64::consts::FRAC_2_PI).round();
    // r = x - k·π/2 with |k| < 2^20. The products k·HALF_PI[0] and
    // k·HALF_PI[1] are formed exactly and their high parts and the larger
    // low part subtracted exactly; what is left (the rounding errors of
    // those subtractions, the other low part and k·HALF_PI[2]) is below
    // 2^-52 (|r| + 2^-32) and is summed with relative error under 2^-50.
    // So r is within 2^-102 |r| + 2^-134 of x - k·(HALF_PI's sum), and
    // that within k·2^-163.5 < 2^-143 of x - k·π/2.
    let (r, reduction_err) = if k == 0.0 {
        ((x, 0.0), 0.0)
    } else {
        let (p1, p1_err) = two_prod(k, HALF_PI[0]);
        let (p2, p2_err) = two_prod(k, HALF_PI[1]);
        let p3 = k * HALF_PI[2];
        let (t, t_err) = two_sum(x, -p1);
        let (s, s_err1) = two_sum(t, -p1_err);
        let (s, s_err2) = two_sum(s, -p2);
        let low = ((t_err + s_err1) + s_err2) - p2_err - p3;
        // The relative part is within the series' bound below.
        (two_sum(s, low), 2f64.powi(-130))
    };
    let quadrant = (k as i64).rem_euclid(4);
    let (use_cos, negate) = match (cosine, quadrant) {
        (false, 0) => (false, false),
        (false, 1) => (true, false),
        (false, 2) => (false, true),
        (false, _) => (true, true),
        (true, 0) => (true, false),
        (true, 1) => (false, true),
        (true, 2) => (true, true),
        (true, _) => (false, false),
    };
    let (hi, lo) = series_dd(r, use_cos);
    // The series' own error, generously 2^-95 relative (about 2^-100 in
    // fact), plus the reduction's, which the derivative (at most 1) carries
    // through unchanged.
    let err = reduction_err + hi.abs() * 2f64.powi(-95);
    let value = round_if_sure(hi, lo, err)?;
    Some(if negate { -value } else { value })
}

/// `x` as `m · 2^e` with `m` an integer below 2^53.
fn decompose(x: f64) -> (u64, i64) {
    let bits = x.abs().to_bits();
    let exponent = (bits >> 52) as i64;
    let fraction = bits & ((1 << 52) - 1);
    if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | 1 << 52, exponent - 1075)
    }
}

/// The sine or cosine of the fixed-point `r` (`r / 2^frac`, at most π/4 +
/// 2^-60), and a bound on its error in units of 2^-frac (for `r` taken as
/// exact).
fn series_fixed(r: &Big, frac: usize, cosine: bool) -> (Big, u64) {
    let r2 = r.mul(r).shr(frac);
    let mut term = if cosine { Big::pow2(frac) } else { r.clone() };
    let mut n: u64 = if cosine { 0 } else { 1 };
    let (mut plus, mut minus) = (term.clone(), Big::zero());
    let mut count = 0;
    loop {
        // Each step truncates twice and scales the error carried in by at
        // most r²/2 < 0.32, so every term is within 5 units; the terms left
        // when one truncates to zero add up to less than 8.
        term = term.mul(&r2).shr(frac).div_u64((n + 1) * (n + 2));
        n += 2;
        if term.is_zero() {
            break;
        }
        count += 1;
        if count % 2 == 1 {
            minus = minus.add(&term);
        } else {
            plus = plus.add(&term);
        }
    }
    (plus.sub(&minus), 6 * (count + 1) + 16)
}

/// The positive fixed-point `v` (`v / 2^frac`) rounded to the nearest
/// double, ties to even; the result must be a normal number.
fn fixed_to_f64(v: &Big, frac: usize) -> f64 {
    let n = v.bits();
    let (mantissa, exponent) = if n > 53 {
        let shift = n - 53;
        let mut m = v.shr(shift).low_u64();
        if v.bit(shift - 1) && (m & 1 == 1 || v.any_below(shift - 1)) {
            m += 1;
        }
        (m, shift as i64 - frac as i64)
    } else {
        (v.low_u64() << (53 - n), n as i64 - 53 - frac as i64)
    };
    debug_assert!((-1022..=1023).contains(&exponent));
    // Both factors and the product are exact.
    mantissa as f64 * f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// The fixed-point path: exact reduction against π/2 to `frac` bits, then
/// the series with `p + 64` fraction bits, for p = 128, 256, … until the
/// error interval rounds to one double.
fn slow(x: f64, cosine: bool) -> f64 {
    let (m, e) = decompose(x);
    // x < 2^top; sin and cos take |x| ≥ 2^-27 here, so e ≥ -79.
    let top = (e + 53).max(0) as usize;
    let mut p = 128;
    loop {
        let series_frac = p + 64;
        let frac = top + series_frac;
        let x_fixed = Big::from_u64(m).shl((e + frac as i64) as usize);
        // (π/2)·2^frac, low by less than 1.
        let half_pi = pi(frac - 1);
        let (k, rem) = x_fixed.div_rem(&half_pi);
        // The nearest multiple: r = x - k·π/2 in [-π/4, π/4] (plus the
        // error), its sign apart.
        let (k, r, r_negative) = if rem.shl(1) > half_pi {
            (k.add(&Big::from_u64(1)), half_pi.sub(&rem), true)
        } else {
            (k, rem, false)
        };
        // The computed r is off by k times π/2's error, below k units; the
        // shift to the series' precision truncates once more.
        let r = r.shr(top);
        let reduction_err = (1u64 << k.bits().saturating_sub(top).min(32)) + 1;
        let quadrant = k.low_u64() & 3;
        let (use_cos, negative) = match (cosine, quadrant) {
            (false, 0) => (false, r_negative),
            (false, 1) => (true, false),
            (false, 2) => (false, !r_negative),
            (false, _) => (true, true),
            (true, 0) => (true, false),
            (true, 1) => (false, !r_negative),
            (true, 2) => (true, true),
            (true, _) => (false, r_negative),
        };
        let (value, series_err) = series_fixed(&r, series_frac, use_cos);
        let err = Big::from_u64(reduction_err + series_err);
        if value > err {
            let low = fixed_to_f64(&value.sub(&err), series_frac);
            let high = fixed_to_f64(&value.add(&err), series_frac);
            if low == high {
                // sin is odd, cos even.
                let negative = negative ^ (!cosine && x < 0.0);
                return if negative { -low } else { low };
            }
        }
        p *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `x` (positive) as a fixed-point number with `frac` fraction bits,
    /// truncated.
    fn to_fixed(x: f64, frac: usize) -> Big {
        let (m, e) = decompose(x);
        let shift = e + frac as i64;
        if shift >= 0 {
            Big::from_u64(m).shl(shift as usize)
        } else {
            Big::from_u64(m).shr((-shift) as usize)
        }
    }

    /// |a - b| for fixed-point numbers.
    fn distance(a: &Big, b: &Big) -> Big {
        if a > b {
            a.sub(b)
        } else {
            b.sub(a)
        }
    }

    #[test]
    fn half_pi_split_is_within_2_pow_minus_160() {
        let frac = 300;
        let exact = pi(frac - 1);
        let parts = HALF_PI.map(|c| to_fixed(c.abs(), frac));
        // HALF_PI[2] is negative.
        let sum = parts[0].add(&parts[1]).sub(&parts[2]);
        assert!(distance(&sum, &exact).bits() <= frac - 160);
    }

    #[test]
    fn inverse_factorials_are_double_double_accurate() {
        let frac = 400;
        let mut factorial = Big::from_u64(1);
        for (n, &(hi, lo)) in INV_FACT.iter().enumerate() {
            if n > 0 {
                factorial = factorial.mul_u64(n as u64);
            }
            let (exact, _) = Big::pow2(frac).div_rem(&factorial);
            let hi_fixed = to_fixed(hi, frac);
            let lo_fixed = to_fixed(lo.abs(), frac);
            let sum = if lo < 0.0 {
                hi_fixed.sub(&lo_fixed)
            } else {
                hi_fixed.add(&lo_fixed)
            };
            // Within 2^-105 of 1/n!, relative.
            assert!(
                distance(&sum, &exact).bits() + 105 <= exact.bits() + 1,
                "1/{n}!"
            );
        }
    }

    #[test]
    fn special_values() {
        assert!(sin(f64::NAN).is_nan() && cos(f64::NAN).is_nan());
        assert!(sin(f64::INFINITY).is_nan() && cos(f64::NEG_INFINITY).is_nan());
        assert!(sin(-0.0).is_sign_negative() && sin(-0.0) == 0.0);
        assert_eq!(cos(-0.0), 1.0);
        assert_eq!(sin(1e-300), 1e-300);
        assert_eq!(sin(-5e-324), -5e-324);
        assert_eq!(cos(1e-9), 1.0);
        // JavaScript's Math.PI / 6 and / 3, not Rust's FRAC_PI_6 and FRAC_PI_3.
        assert_eq!(sin(std::f64::consts::PI / 6.0), 0.49999999999999994);
        assert_eq!(cos(std::f64::consts::PI / 3.0), 0.5000000000000001);
        assert_eq!(sin(std::f64::consts::PI), 1.2246467991473532e-16);
        assert_eq!(cos(std::f64::consts::FRAC_PI_2), 6.123233995736766e-17);
    }

    /// A deterministic sequence of doubles spread over the fast path's
    /// range, and of the doubles nearest multiples of π/2 there.
    fn samples(count: usize) -> Vec<f64> {
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        (0..count)
            .map(|i| {
                let u = (next() >> 11) as f64 / (1u64 << 53) as f64;
                match i % 3 {
                    0 => (u * 2.0 - 1.0) * 8.0,
                    1 => (u * 2.0 - 1.0) * 2f64.powi((next() % 21) as i32),
                    _ => ((next() % 600_000) as f64) * std::f64::consts::FRAC_PI_2,
                }
            })
            .collect()
    }

    /// `TRIG_SAMPLES` sets the count (1,500 by default), for longer runs:
    /// `TRIG_SAMPLES=3000000 cargo test --release -p js --lib fast_path`.
    #[test]
    fn fast_path_agrees_with_the_fixed_point_path() {
        let count = std::env::var("TRIG_SAMPLES").map_or(1500, |n| n.parse().unwrap());
        let mut undecided = 0;
        for x in samples(count) {
            if x.abs() < TWO_M26 {
                continue;
            }
            for cosine in [false, true] {
                match fast(x, cosine) {
                    Some(fast) => assert_eq!(
                        fast.to_bits(),
                        slow(x, cosine).to_bits(),
                        "{}({x:?})",
                        if cosine { "cos" } else { "sin" }
                    ),
                    None => undecided += 1,
                }
            }
        }
        println!("{count} samples: {undecided} of {} undecided", 2 * count);
        assert!(undecided * 1000 < count, "{undecided} undecided");
    }

    #[test]
    fn the_hardest_reduction() {
        // 6381956970095103·2^797 is the double nearest a multiple of π/2
        // (within about 2^-60.9). Expected values: Python's decimal module
        // at 80 digits after an exact reduction.
        let x = 6381956970095103.0 * 2f64.powi(797);
        assert_eq!(cos(x), -4.687165924254628e-19);
        assert_eq!(sin(x), 1.0);
    }
}
