//! `Math.asin` and `Math.tan` as V8 computes them: its fdlibm port,
//! `src/base/ieee754.cc` at V8 14.7 (Chromium 147), which Node 24's V8
//! shares. Chrome 147 and Node 24 gave the same `asin` and `tan` on every
//! argument measured (`tests/trig.rs` checks the port against Chrome).
//!
//! Translated line by line, keeping fdlibm's names, constants and C integer
//! semantics (32-bit words, truncating conversions). The original carries
//! this notice:
//!
//! > Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Developed at SunSoft, a Sun Microsystems, Inc. business.
//! > Permission to use, copy, modify, and distribute this
//! > software is freely granted, provided that this notice
//! > is preserved.
//!
//! and V8's changes are "Copyright 2016 the V8 project authors", under V8's
//! BSD licence.

// fdlibm writes `x - x` for the NaN of an infinite or NaN argument.
#![allow(clippy::excessive_precision, clippy::approx_constant, clippy::eq_op)]

fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

fn low_word(x: f64) -> u32 {
    x.to_bits() as u32
}

fn with_low_word(x: f64, low: u32) -> f64 {
    f64::from_bits((x.to_bits() & 0xFFFF_FFFF_0000_0000) | u64::from(low))
}

fn with_high_word(x: f64, high: i32) -> f64 {
    f64::from_bits((x.to_bits() & 0xFFFF_FFFF) | (u64::from(high as u32) << 32))
}

/// `scalbn(x, n)` for the exponents used here, where the scaling is exact.
fn scalbn(x: f64, n: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&n));
    x * f64::from_bits(((n + 1023) as u64) << 52)
}

/// C's `(int32_t)d`: truncation toward zero (the values here fit).
fn trunc_i32(d: f64) -> i32 {
    d as i32
}

/// `__ieee754_rem_pio2(x, y)`: x - n·π/2 in `y[0] + y[1]`, returning n.
fn rem_pio2(x: f64, y: &mut [f64; 2]) -> i32 {
    // 396 hexadecimal digits of 2/π, 24 bits each.
    const TWO_OVER_PI: [i32; 66] = [
        0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
        0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
        0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
        0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
        0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
        0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
        0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
        0x73A8C9, 0x60E27B, 0xC08C6B,
    ];
    const NPIO2_HW: [i32; 32] = [
        0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB,
        0x402921FB, 0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB,
        0x40378FDB, 0x403921FB, 0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C,
        0x4041475C, 0x4042106C, 0x4042D97C, 0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB,
        0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
    ];
    const ZERO: f64 = 0.00000000000000000000e+00;
    const HALF: f64 = 5.00000000000000000000e-01;
    const TWO24: f64 = 1.67772160000000000000e+07;
    const INVPIO2: f64 = 6.36619772367581382433e-01;
    const PIO2_1: f64 = 1.57079632673412561417e+00;
    const PIO2_1T: f64 = 6.07710050650619224932e-11;
    const PIO2_2: f64 = 6.07710050630396597660e-11;
    const PIO2_2T: f64 = 2.02226624879595063154e-21;
    const PIO2_3: f64 = 2.02226624871116645580e-21;
    const PIO2_3T: f64 = 8.47842766036889956997e-32;

    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix <= 0x3FE921FB {
        y[0] = x;
        y[1] = 0.0;
        return 0;
    }
    if ix < 0x4002D97C {
        // |x| < 3π/4: n = ±1.
        if hx > 0 {
            let mut z = x - PIO2_1;
            if ix != 0x3FF921FB {
                y[0] = z - PIO2_1T;
                y[1] = (z - y[0]) - PIO2_1T;
            } else {
                z -= PIO2_2;
                y[0] = z - PIO2_2T;
                y[1] = (z - y[0]) - PIO2_2T;
            }
            return 1;
        } else {
            let mut z = x + PIO2_1;
            if ix != 0x3FF921FB {
                y[0] = z + PIO2_1T;
                y[1] = (z - y[0]) + PIO2_1T;
            } else {
                z += PIO2_2;
                y[0] = z + PIO2_2T;
                y[1] = (z - y[0]) + PIO2_2T;
            }
            return -1;
        }
    }
    if ix <= 0x413921FB {
        // |x| ~<= 2^19·(π/2): medium size.
        let t = x.abs();
        let n = trunc_i32(t * INVPIO2 + HALF);
        let fn_ = f64::from(n);
        let mut r = t - fn_ * PIO2_1;
        let mut w = fn_ * PIO2_1T;
        if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
            y[0] = r - w;
        } else {
            let j = ix >> 20;
            y[0] = r - w;
            let high = high_word(y[0]) as u32;
            let mut i = j - ((high >> 20) & 0x7FF) as i32;
            if i > 16 {
                let t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fn_ * PIO2_2T - ((t - r) - w);
                y[0] = r - w;
                let high = high_word(y[0]) as u32;
                i = j - ((high >> 20) & 0x7FF) as i32;
                if i > 49 {
                    let t = r;
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fn_ * PIO2_3T - ((t - r) - w);
                    y[0] = r - w;
                }
            }
        }
        y[1] = (r - y[0]) - w;
        if hx < 0 {
            y[0] = -y[0];
            y[1] = -y[1];
            return -n;
        }
        return n;
    }
    if ix >= 0x7FF00000 {
        y[0] = x - x;
        y[1] = y[0];
        return 0;
    }
    // z = scalbn(|x|, ilogb(x) - 23)
    let low = low_word(x);
    let mut z = with_low_word(0.0, low);
    let e0 = (ix >> 20) - 1046;
    z = with_high_word(z, ix - ((e0 as u32) << 20) as i32);
    let mut tx = [0.0; 3];
    for t in tx.iter_mut().take(2) {
        *t = f64::from(trunc_i32(z));
        z = (z - *t) * TWO24;
    }
    tx[2] = z;
    let mut nx = 3;
    while tx[nx - 1] == ZERO {
        nx -= 1;
    }
    let n = kernel_rem_pio2(&tx[..nx], y, e0, &TWO_OVER_PI);
    if hx < 0 {
        y[0] = -y[0];
        y[1] = -y[1];
        return -n;
    }
    n
}

/// `__kernel_rem_pio2(x, y, e0, nx, 2, ipio2)`, at the precision `rem_pio2`
/// asks for (prec 2: jk = 4, two output terms).
fn kernel_rem_pio2(x: &[f64], y: &mut [f64; 2], e0: i32, ipio2: &[i32]) -> i32 {
    const PIO2: [f64; 8] = [
        1.57079625129699707031e+00,
        7.54978941586159635335e-08,
        5.39030252995776476554e-15,
        3.28200341580791294123e-22,
        1.27065575308067607349e-29,
        1.22933308981111328932e-36,
        2.73370053816464559624e-44,
        2.16741683877804819444e-51,
    ];
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO24: f64 = 1.67772160000000000000e+07;
    const TWON24: f64 = 5.96046447753906250000e-08;

    let nx = x.len() as i32;
    let jk: i32 = 4;
    let jp = jk;
    let mut iq = [0i32; 20];
    let mut f = [0.0f64; 20];
    let mut fq = [0.0f64; 20];
    let mut q = [0.0f64; 20];

    let jx = nx - 1;
    let mut jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    let mut q0 = e0 - 24 * (jv + 1);

    let mut j = jv - jx;
    let m = jx + jk;
    for slot in f.iter_mut().take((m + 1) as usize) {
        *slot = if j < 0 {
            ZERO
        } else {
            f64::from(ipio2[j as usize])
        };
        j += 1;
    }

    for i in 0..=jk {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw += x[j as usize] * f[(jx + i - j) as usize];
        }
        q[i as usize] = fw;
    }

    let mut jz = jk;
    let (mut z, mut n, ih) = loop {
        // Distill q[] into iq[] reversingly.
        let mut z = q[jz as usize];
        let mut i = 0usize;
        let mut j = jz;
        while j > 0 {
            let fw = f64::from(trunc_i32(TWON24 * z));
            iq[i] = trunc_i32(z - TWO24 * fw);
            z = q[(j - 1) as usize] + fw;
            i += 1;
            j -= 1;
        }

        // Compute n.
        z = scalbn(z, q0);
        z -= 8.0 * (z * 0.125).floor();
        let mut n = trunc_i32(z);
        z -= f64::from(n);
        let mut ih = 0;
        if q0 > 0 {
            let i = iq[(jz - 1) as usize] >> (24 - q0);
            n += i;
            iq[(jz - 1) as usize] -= i << (24 - q0);
            ih = iq[(jz - 1) as usize] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[(jz - 1) as usize] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            n += 1;
            let mut carry = 0;
            for slot in iq.iter_mut().take(jz as usize) {
                let j = *slot;
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        *slot = 0x1000000 - j;
                    }
                } else {
                    *slot = 0xFFFFFF - j;
                }
            }
            if q0 > 0 {
                match q0 {
                    1 => iq[(jz - 1) as usize] &= 0x7FFFFF,
                    2 => iq[(jz - 1) as usize] &= 0x3FFFFF,
                    _ => {}
                }
            }
            if ih == 2 {
                z = ONE - z;
                if carry != 0 {
                    z -= scalbn(ONE, q0);
                }
            }
        }

        // Check whether recomputation is needed.
        if z == ZERO {
            let mut j = 0;
            let mut i = jz - 1;
            while i >= jk {
                j |= iq[i as usize];
                i -= 1;
            }
            if j == 0 {
                let mut k = 1;
                while jk >= k && iq[(jk - k) as usize] == 0 {
                    k += 1;
                }
                for i in (jz + 1)..=(jz + k) {
                    f[(jx + i) as usize] = f64::from(ipio2[(jv + i) as usize]);
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw += x[j as usize] * f[(jx + i - j) as usize];
                    }
                    q[i as usize] = fw;
                }
                jz += k;
                continue;
            }
        }
        break (z, n, ih);
    };

    // Chop off zero terms.
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz as usize] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        z = scalbn(z, -q0);
        if z >= TWO24 {
            let fw = f64::from(trunc_i32(TWON24 * z));
            iq[jz as usize] = trunc_i32(z - TWO24 * fw);
            jz += 1;
            q0 += 24;
            iq[jz as usize] = trunc_i32(fw);
        } else {
            iq[jz as usize] = trunc_i32(z);
        }
    }

    // Convert the integer chunks to floating point.
    let mut fw = scalbn(ONE, q0);
    let mut i = jz;
    while i >= 0 {
        q[i as usize] = fw * f64::from(iq[i as usize]);
        fw *= TWON24;
        i -= 1;
    }

    // PIo2[0..=jp] · q[jz..=0].
    let mut i = jz;
    while i >= 0 {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= jp && k <= jz - i {
            fw += PIO2[k as usize] * q[(i + k) as usize];
            k += 1;
        }
        fq[(jz - i) as usize] = fw;
        i -= 1;
    }

    // Compress fq[] into y[] (prec 2).
    let mut fw = 0.0;
    let mut i = jz;
    while i >= 0 {
        fw += fq[i as usize];
        i -= 1;
    }
    y[0] = if ih == 0 { fw } else { -fw };
    let mut fw = fq[0] - fw;
    for i in 1..=jz {
        fw += fq[i as usize];
    }
    y[1] = if ih == 0 { fw } else { -fw };
    n &= 7;
    n
}

/// `__kernel_tan(x, y, iy)`: tan(x + y) for iy = 1, -1/tan(x + y) for
/// iy = -1, |x| ≤ π/4.
fn kernel_tan(mut x: f64, mut y: f64, iy: i32) -> f64 {
    const T: [f64; 13] = [
        3.33333333333334091986e-01,
        1.33333333333201242699e-01,
        5.39682539762260521377e-02,
        2.18694882948595424599e-02,
        8.86323982359930005737e-03,
        3.59207910759131235356e-03,
        1.45620945432529025516e-03,
        5.88041240820264096874e-04,
        2.46463134818469906812e-04,
        7.81794442939557092300e-05,
        7.14072491382608190305e-05,
        -1.85586374855275456654e-05,
        2.59073051863633712884e-05,
    ];
    const ONE: f64 = 1.00000000000000000000e+00;
    const PIO4: f64 = 7.85398163397448278999e-01;
    const PIO4LO: f64 = 3.06161699786838301793e-17;

    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix < 0x3E300000 && trunc_i32(x) == 0 {
        // |x| < 2^-28
        let low = low_word(x);
        if ((ix as u32 | low) as i32 | (iy + 1)) == 0 {
            return ONE / x.abs();
        } else if iy == 1 {
            return x;
        } else {
            // -1 / (x + y), carefully.
            let w = x + y;
            let z = with_low_word(w, 0);
            let v = y - (z - x);
            let a = -ONE / w;
            let t = with_low_word(a, 0);
            let s = ONE + t * z;
            return t + a * (s + t * v);
        }
    }
    if ix >= 0x3FE59428 {
        // |x| ≥ 0.6744
        if hx < 0 {
            x = -x;
            y = -y;
        }
        let z = PIO4 - x;
        let w = PIO4LO - y;
        x = z + w;
        y = 0.0;
    }
    let z = x * x;
    let w = z * z;
    let mut r = T[1] + w * (T[3] + w * (T[5] + w * (T[7] + w * (T[9] + w * T[11]))));
    let v = z * (T[2] + w * (T[4] + w * (T[6] + w * (T[8] + w * (T[10] + w * T[12])))));
    let s = z * x;
    r = y + z * (s * (r + v) + y);
    r += T[0] * s;
    let w = x + r;
    if ix >= 0x3FE59428 {
        let v = f64::from(iy);
        return f64::from(1 - ((hx >> 30) & 2)) * (v - 2.0 * (x - (w * w / (w + v) - r)));
    }
    if iy == 1 {
        w
    } else {
        // -1.0 / (x + r), accurately.
        let z = with_low_word(w, 0);
        let v = r - (z - x);
        let a = -1.0 / w;
        let t = with_low_word(a, 0);
        let s = 1.0 + t * z;
        t + a * (s + t * v)
    }
}

/// `Math.tan(x)`.
pub fn tan(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE921FB {
        kernel_tan(x, 0.0, 1)
    } else if ix >= 0x7FF00000 {
        x - x
    } else {
        let mut y = [0.0; 2];
        let n = rem_pio2(x, &mut y);
        kernel_tan(y[0], y[1], 1 - ((n & 1) << 1))
    }
}

/// `Math.asin(x)`.
pub fn asin(x: f64) -> f64 {
    const ONE: f64 = 1.00000000000000000000e+00;
    const HUGE: f64 = 1.000e+300;
    const PIO2_HI: f64 = 1.57079632679489655800e+00;
    const PIO2_LO: f64 = 6.12323399573676603587e-17;
    const PIO4_HI: f64 = 7.85398163397448278999e-01;
    const PS0: f64 = 1.66666666666666657415e-01;
    const PS1: f64 = -3.25565818622400915405e-01;
    const PS2: f64 = 2.01212532134862925881e-01;
    const PS3: f64 = -4.00555345006794114027e-02;
    const PS4: f64 = 7.91534994289814532176e-04;
    const PS5: f64 = 3.47933107596021167570e-05;
    const QS1: f64 = -2.40339491173441421878e+00;
    const QS2: f64 = 2.02094576023350569471e+00;
    const QS3: f64 = -6.88283971605453293030e-01;
    const QS4: f64 = 7.70381505559019352791e-02;

    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    let mut t = 0.0;
    if ix >= 0x3FF00000 {
        // |x| ≥ 1
        let lx = low_word(x);
        if ((ix - 0x3FF00000) as u32 | lx) == 0 {
            return x * PIO2_HI + x * PIO2_LO;
        }
        return f64::NAN;
    } else if ix < 0x3FE00000 {
        // |x| < 0.5
        if ix < 0x3E400000 {
            if HUGE + x > ONE {
                return x;
            }
        } else {
            t = x * x;
        }
        let p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * (PS4 + t * PS5)))));
        let q = ONE + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
        let w = p / q;
        return x + x * w;
    }
    // 1 > |x| ≥ 0.5
    let w = ONE - x.abs();
    t = w * 0.5;
    let mut p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * (PS4 + t * PS5)))));
    let mut q = ONE + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
    let s = t.sqrt();
    if ix >= 0x3FEF3333 {
        // |x| > 0.975
        let w = p / q;
        t = PIO2_HI - (2.0 * (s + s * w) - PIO2_LO);
    } else {
        let w = with_low_word(s, 0);
        let c = (t - w * w) / (s + w);
        let r = p / q;
        p = 2.0 * s * r - (PIO2_LO - 2.0 * c);
        q = PIO4_HI - 2.0 * w;
        t = PIO4_HI - (p - q);
    }
    if hx > 0 {
        t
    } else {
        -t
    }
}
