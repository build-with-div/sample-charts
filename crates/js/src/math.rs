//! `Math` functions whose results differ between implementations, computed
//! the way V8 computes them, so ported code gets Chromium's numbers bit for
//! bit. (`Math.floor`, `ceil`, `sqrt`, `abs` are exact everywhere and map to
//! Rust's.)
//!
//! - `Math.log` and `Math.log10` are V8's ports of fdlibm
//!   (`src/base/ieee754.cc`, unchanged between V8 12.4 in Node 22 and V8 14.7
//!   in Chromium 147). Neither Rust's `f64::log10` (the C library) nor the
//!   `libm` crate gives the same last bit for every input.
//! - `Math.pow` in V8 14.7 is `v8::internal::math::pow`
//!   (`src/numbers/ieee754.cc`): with `--use-std-math-pow`, on by default
//!   since V8 13.x, it handles a few cases itself and otherwise calls the C
//!   library's `pow`, as Rust's `f64::powf` does. So Chromium's results
//!   depend on the platform's C library, and so do these. (V8 12.4, in
//!   Node 22, still used fdlibm's `pow`; the fixtures record the `Math.pow`
//!   calls they depend on so the tests can tell.) Measured on Windows 11:
//!   Chrome for Testing 147.0.7727.50 and this function agree on
//!   `Math.pow(10, n)` for every integral n in -340..=340 except 126, where
//!   the `ucrtbase.dll` pow Rust links gives 1.0000000000000001e126 and
//!   Chrome 1e126; both give 1.0000000000000001e23 for n = 23.
//!
//! - `Math.sin` and `Math.cos` are correctly rounded (`trig.rs`), not V8's
//!   bundled code (not ported, for its licence); they equal Chromium 147's
//!   except where Chromium's are not correctly rounded (`tests/trig.rs`).
//!
//! The fdlibm constants are written as V8's source writes them, so the port
//! can be compared line by line; their literals carry more digits than an
//! `f64` holds, as in C.

#![allow(clippy::excessive_precision, clippy::approx_constant)]

mod big;
mod trig;

pub use trig::{cos, sin};

/// `Math.max(a, b)`: NaN if either is NaN, and +0 over -0.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_positive() {
            a
        } else {
            b
        }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(a, b)`: NaN if either is NaN, and -0 over +0.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() {
            a
        } else {
            b
        }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(...values)`: -Infinity for none.
pub fn max_of(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::NEG_INFINITY, max)
}

/// `Math.min(...values)`: Infinity for none.
pub fn min_of(values: impl IntoIterator<Item = f64>) -> f64 {
    values.into_iter().fold(f64::INFINITY, min)
}

/// `Math.pow(x, y)` and `x ** y`, as `v8::internal::math::pow` in V8 14.7.
pub fn pow(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        return f64::NAN;
    }
    if y.is_infinite() && (x == 1.0 || x == -1.0) {
        return f64::NAN;
    }
    if y == 2.0 {
        return x * x;
    }
    if y == 0.5 {
        if x.is_infinite() {
            return f64::INFINITY;
        }
        return (x + 0.0).sqrt();
    }
    x.powf(y)
}

fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

fn low_word(x: f64) -> u32 {
    x.to_bits() as u32
}

fn with_high_word(x: f64, high: i32) -> f64 {
    f64::from_bits(((high as u32 as u64) << 32) | (x.to_bits() & 0xFFFF_FFFF))
}

fn with_words(high: i32, low: u32) -> f64 {
    f64::from_bits(((high as u32 as u64) << 32) | low as u64)
}

/// `Math.log(x)`: fdlibm's `__ieee754_log` as V8's `base::ieee754::log`.
pub fn log(x: f64) -> f64 {
    const LN2_HI: f64 = 6.93147180369123816490e-01;
    const LN2_LO: f64 = 1.90821492927058770002e-10;
    const TWO54: f64 = 1.80143985094819840000e+16;
    const LG1: f64 = 6.666666666666735130e-01;
    const LG2: f64 = 3.999999999940941908e-01;
    const LG3: f64 = 2.857142874366239149e-01;
    const LG4: f64 = 2.222219843214978396e-01;
    const LG5: f64 = 1.818357216161805012e-01;
    const LG6: f64 = 1.531383769920937332e-01;
    const LG7: f64 = 1.479819860511658591e-01;

    let mut x = x;
    let mut hx = high_word(x);
    let lx = low_word(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        if ((hx & 0x7FFF_FFFF) as u32 | lx) == 0 {
            return f64::NEG_INFINITY;
        }
        if hx < 0 {
            return f64::NAN;
        }
        k -= 54;
        x *= TWO54;
        hx = high_word(x);
    }
    if hx >= 0x7FF0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000F_FFFF;
    let i = (hx + 0x95F64) & 0x10_0000;
    x = with_high_word(x, hx | (i ^ 0x3FF0_0000));
    k += i >> 20;
    let f = x - 1.0;
    if (0x000F_FFFF & (2 + hx)) < 3 {
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk = f64::from(k);
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        if k == 0 {
            return f - r;
        }
        let dk = f64::from(k);
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = f64::from(k);
    let z = s * s;
    let mut i = hx - 0x6147A;
    let w = z * z;
    let j = 0x6B851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

/// `Math.log10(x)`: fdlibm's `__ieee754_log10` as V8's
/// `base::ieee754::log10`.
pub fn log10(x: f64) -> f64 {
    const TWO54: f64 = 1.80143985094819840000e+16;
    const IVLN10: f64 = 4.34294481903251816668e-01;
    const LOG10_2HI: f64 = 3.01029995663611771306e-01;
    const LOG10_2LO: f64 = 3.69423907715893078616e-13;

    let mut x = x;
    let mut hx = high_word(x);
    let mut lx = low_word(x);
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        if ((hx & 0x7FFF_FFFF) as u32 | lx) == 0 {
            return f64::NEG_INFINITY;
        }
        if hx < 0 {
            return f64::NAN;
        }
        k -= 54;
        x *= TWO54;
        hx = high_word(x);
        lx = low_word(x);
    }
    if hx >= 0x7FF0_0000 {
        return x + x;
    }
    if hx == 0x3FF0_0000 && lx == 0 {
        return 0.0;
    }
    k += (hx >> 20) - 1023;
    let i = ((k as u32 & 0x8000_0000) >> 31) as i32;
    hx = (hx & 0x000F_FFFF) | ((0x3FF - i) << 20);
    let y = f64::from(k + i);
    x = with_words(hx, lx);
    let z = y * LOG10_2LO + IVLN10 * log(x);
    z + y * LOG10_2HI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log10_of_powers_of_ten_is_exact() {
        // fdlibm guarantees log10(10**N) = N for N = 0..=22.
        for n in 0..=22 {
            let x: f64 = format!("1e{n}").parse().unwrap();
            assert_eq!(log10(x), f64::from(n), "log10(1e{n})");
        }
    }

    #[test]
    fn special_values() {
        assert_eq!(log10(0.0), f64::NEG_INFINITY);
        assert_eq!(log10(-0.0), f64::NEG_INFINITY);
        assert!(log10(-1.0).is_nan());
        assert_eq!(log10(f64::INFINITY), f64::INFINITY);
        assert_eq!(log(1.0), 0.0);
        assert!(pow(1.0, f64::INFINITY).is_nan());
        assert_eq!(pow(f64::NEG_INFINITY, 0.5), f64::INFINITY);
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert_eq!(max_of([]), f64::NEG_INFINITY);
    }
}
