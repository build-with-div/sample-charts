//! `d3.scaleBand` and `d3.scalePoint`: `src/band.js` of d3-scale 4.0.3, on
//! the ordinal scale of `src/ordinal.js` with an `undefined` unknown value.
//! Domain values are keyed as d3's `InternMap` keys primitives, by
//! SameValueZero ([`js::Key`]); the domain keeps the values as given, so a
//! `-0` stays `-0`.

use std::collections::HashMap;

use js::{math, math_round, Key, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    domain: Vec<Value>,
    index: HashMap<Key, usize>,
    ordinal_range: Vec<f64>,
    r0: f64,
    r1: f64,
    step: f64,
    bandwidth: f64,
    round: bool,
    padding_inner: f64,
    padding_outer: f64,
    align: f64,
}

impl Default for Band {
    fn default() -> Self {
        Self::new()
    }
}

impl Band {
    /// `d3.scaleBand()`.
    pub fn new() -> Self {
        let mut band = Self {
            domain: Vec::new(),
            index: HashMap::new(),
            ordinal_range: Vec::new(),
            r0: 0.0,
            r1: 1.0,
            step: 0.0,
            bandwidth: 0.0,
            round: false,
            padding_inner: 0.0,
            padding_outer: 0.0,
            align: 0.5,
        };
        band.rescale();
        band
    }

    /// `d3.scalePoint()`: a band scale with `paddingInner(1)`.
    pub fn point() -> Self {
        let mut point = Self::new();
        point.set_padding_inner(1.0);
        point
    }

    fn rescale(&mut self) {
        let n = self.domain.len() as f64;
        let reverse = self.r1 < self.r0;
        let (mut start, stop) = if reverse {
            (self.r1, self.r0)
        } else {
            (self.r0, self.r1)
        };
        let mut step =
            (stop - start) / math::max(1.0, n - self.padding_inner + self.padding_outer * 2.0);
        if self.round {
            step = step.floor();
        }
        start += (stop - start - step * (n - self.padding_inner)) * self.align;
        let mut bandwidth = step * (1.0 - self.padding_inner);
        if self.round {
            start = math_round(start);
            bandwidth = math_round(bandwidth);
        }
        self.step = step;
        self.bandwidth = bandwidth;
        let mut values: Vec<f64> = (0..self.domain.len())
            .map(|i| start + step * i as f64)
            .collect();
        if reverse {
            values.reverse();
        }
        self.ordinal_range = values;
    }

    pub fn domain(&self) -> &[Value] {
        &self.domain
    }

    /// `scale.domain(values)`: repeated values keep their first position.
    /// Arrays and objects, which `InternMap` keys by identity, are not
    /// modelled: the first one is returned as the error and the scale is
    /// left unchanged.
    pub fn set_domain(
        &mut self,
        values: impl IntoIterator<Item = Value>,
    ) -> Result<&mut Self, Value> {
        let mut domain = Vec::new();
        let mut index = HashMap::new();
        for value in values {
            let Some(key) = Key::of(&value) else {
                return Err(value);
            };
            if index.contains_key(&key) {
                continue;
            }
            index.insert(key, domain.len());
            domain.push(value);
        }
        self.domain = domain;
        self.index = index;
        self.rescale();
        Ok(self)
    }

    /// `scale.range()`: `[r0, r1]`.
    pub fn range(&self) -> [f64; 2] {
        [self.r0, self.r1]
    }

    pub fn set_range(&mut self, range: [f64; 2]) -> &mut Self {
        [self.r0, self.r1] = range;
        self.rescale();
        self
    }

    pub fn set_range_round(&mut self, range: [f64; 2]) -> &mut Self {
        [self.r0, self.r1] = range;
        self.round = true;
        self.rescale();
        self
    }

    pub fn set_round(&mut self, round: bool) -> &mut Self {
        self.round = round;
        self.rescale();
        self
    }

    /// `band.padding(p)`: both inner (at most 1) and outer padding.
    pub fn set_padding(&mut self, padding: f64) -> &mut Self {
        self.padding_outer = padding;
        self.padding_inner = math::min(1.0, padding);
        self.rescale();
        self
    }

    pub fn set_padding_inner(&mut self, padding: f64) -> &mut Self {
        self.padding_inner = math::min(1.0, padding);
        self.rescale();
        self
    }

    /// `band.paddingOuter(p)`, which is `point.padding(p)`.
    pub fn set_padding_outer(&mut self, padding: f64) -> &mut Self {
        self.padding_outer = padding;
        self.rescale();
        self
    }

    pub fn set_align(&mut self, align: f64) -> &mut Self {
        self.align = math::max(0.0, math::min(1.0, align));
        self.rescale();
        self
    }

    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }

    pub fn step(&self) -> f64 {
        self.step
    }

    /// `scale(value)`: the start of the value's band, `None` (`undefined`)
    /// for values not in the domain.
    pub fn apply(&self, value: &Value) -> Option<f64> {
        let i = *self.index.get(&Key::of(value)?)?;
        if self.ordinal_range.is_empty() {
            return None;
        }
        Some(self.ordinal_range[i % self.ordinal_range.len()])
    }
}
