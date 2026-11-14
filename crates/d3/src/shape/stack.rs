//! d3-shape 3.2.1's `stack()` with `stackOrderNone` and the offsets
//! `stackOffsetNone`, `stackOffsetExpand`, `stackOffsetSilhouette` and
//! `stackOffsetWiggle` (`src/stack.js`, `src/offset/*.js`,
//! `src/order/none.js`).
//!
//! The values come in already read, as `+value(d, key, j, data)` gives them:
//! `values[i][j]` is series `i` (key `i`) at datum `j`. Every series has one
//! point per datum, so all have the same length.

/// One series: `[y0, y1]` per datum.
pub type Series = Vec<[f64; 2]>;

/// `series[i][j][1] || 0`: NaN and zero read as zero.
fn or_zero(value: f64) -> f64 {
    if value.is_nan() || value == 0.0 {
        0.0
    } else {
        value
    }
}

/// `stackOrderNone(series)`: the series in key order.
pub fn order_none(n: usize) -> Vec<usize> {
    (0..n).collect()
}

/// `stack().keys(keys).value(value).order(stackOrderNone).offset(offset)(data)`:
/// each point starts as `[0, value]`, then `offset` moves it. Returns the
/// series in key order (`series.index` is the position in the order, which
/// for `stackOrderNone` is the key index).
pub fn stack(values: &[Vec<f64>], offset: impl FnOnce(&mut [Series], &[usize])) -> Vec<Series> {
    let mut series: Vec<Series> = values
        .iter()
        .map(|points| points.iter().map(|v| [0.0, *v]).collect())
        .collect();
    let order = order_none(series.len());
    offset(&mut series, &order);
    series
}

/// `stackOffsetNone`: each series sits on the one before it in the order.
pub fn offset_none(series: &mut [Series], order: &[usize]) {
    let n = series.len();
    if n <= 1 {
        return;
    }
    let m = series[order[0]].len();
    for i in 1..n {
        // isNaN(s0[j][1]) ? s0[j][0] : s0[j][1]
        let bases: Vec<f64> = series[order[i - 1]]
            .iter()
            .take(m)
            .map(|below| {
                if below[1].is_nan() {
                    below[0]
                } else {
                    below[1]
                }
            })
            .collect();
        for (point, base) in series[order[i]].iter_mut().zip(bases) {
            point[0] = base;
            point[1] += base;
        }
    }
}

/// `stackOffsetExpand`: each datum's values are divided by their sum, then
/// stacked as by `stackOffsetNone`.
pub fn offset_expand(series: &mut [Series], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let m = series[0].len();
    for j in 0..m {
        let mut y = 0.0;
        for s in series.iter() {
            y += or_zero(s[j][1]);
        }
        // `if (y)`
        if y != 0.0 && !y.is_nan() {
            for s in series.iter_mut() {
                s[j][1] /= y;
            }
        }
    }
    offset_none(series, order);
}

/// `stackOffsetSilhouette`: the stack centred on zero.
pub fn offset_silhouette(series: &mut [Series], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let first = order[0];
    let m = series[first].len();
    for j in 0..m {
        let mut y = 0.0;
        for s in series.iter() {
            y += or_zero(s[j][1]);
        }
        let base = -y / 2.0;
        series[first][j][0] = base;
        series[first][j][1] += base;
    }
    offset_none(series, order);
}

/// `stackOffsetWiggle`: the baseline that minimises the weighted wiggle
/// (Byron and Wattenberg's streamgraph).
pub fn offset_wiggle(series: &mut [Series], order: &[usize]) {
    let n = series.len();
    if n == 0 {
        return;
    }
    let first = order[0];
    let m = series[first].len();
    if m == 0 {
        return;
    }
    let mut y = 0.0;
    for j in 1..m {
        let (mut s1, mut s2) = (0.0, 0.0);
        for i in 0..n {
            let si = &series[order[i]];
            let sij0 = or_zero(si[j][1]);
            let sij1 = or_zero(si[j - 1][1]);
            let mut s3 = (sij0 - sij1) / 2.0;
            for &k in &order[..i] {
                let sk = &series[k];
                s3 += or_zero(sk[j][1]) - or_zero(sk[j - 1][1]);
            }
            s1 += sij0;
            s2 += s3 * sij0;
        }
        series[first][j - 1][0] = y;
        series[first][j - 1][1] += y;
        // `if (s1)`
        if s1 != 0.0 && !s1.is_nan() {
            y -= s2 / s1;
        }
    }
    series[first][m - 1][0] = y;
    series[first][m - 1][1] += y;
    offset_none(series, order);
}
