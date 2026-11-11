use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
}

impl Interval {
    pub fn new(lo: f64, hi: f64) -> Self {
        if lo > hi {
            Self { lo: hi, hi: lo }
        } else {
            Self { lo, hi }
        }
    }

    pub fn singleton(x: f64) -> Self {
        Self { lo: x, hi: x }
    }

    pub fn contains(&self, val: f64) -> bool {
        self.lo <= val && val <= self.hi
    }

    pub fn width(&self) -> f64 {
        self.hi - self.lo
    }

    pub fn abs(&self) -> f64 {
        self.lo.abs().max(self.hi.abs())
    }

    pub fn intersect(&self, other: &Interval) -> Option<Interval> {
        let lo = self.lo.max(other.lo);
        let hi = self.hi.min(other.hi);
        if lo <= hi {
            Some(Interval::new(lo, hi))
        } else {
            None
        }
    }

    pub fn hull(&self, other: &Interval) -> Interval {
        Interval::new(self.lo.min(other.lo), self.hi.max(other.hi))
    }

    pub fn add(&self, other: &Interval) -> Interval {
        Interval::new(self.lo + other.lo, self.hi + other.hi)
    }

    pub fn sub(&self, other: &Interval) -> Interval {
        Interval::new(self.lo - other.hi, self.hi - other.lo)
    }

    pub fn mul(&self, other: &Interval) -> Interval {
        let (a, b, c, d) = (self.lo, self.hi, other.lo, other.hi);
        let vals = [a * c, a * d, b * c, b * d];
        Interval::new(
            vals.iter().cloned().fold(f64::INFINITY, f64::min),
            vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    pub fn div(&self, other: &Interval) -> Interval {
        if other.contains(0.0) {
            if other.lo == 0.0 && other.hi == 0.0 {
                return Interval::new(f64::NEG_INFINITY, f64::INFINITY);
            }
            let left = if other.lo < 0.0 {
                Some(self.div(&Interval::new(other.lo, -0.0_f64.min(-other.hi))))
            } else {
                None
            };
            let right = if other.hi > 0.0 {
                Some(self.div(&Interval::new(0.0_f64.max(-other.lo), other.hi)))
            } else {
                None
            };
            match (left, right) {
                (Some(l), Some(r)) => l.hull(&r),
                (Some(l), None) => l,
                (None, Some(r)) => r,
                (None, None) => Interval::new(f64::NEG_INFINITY, f64::INFINITY),
            }
        } else {
            let (a, b, c, d) = (self.lo, self.hi, other.lo, other.hi);
            let vals = [a / c, a / d, b / c, b / d];
            Interval::new(
                vals.iter().cloned().fold(f64::INFINITY, f64::min),
                vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        }
    }

    pub fn sin(&self) -> Interval {
        let pi2 = std::f64::consts::PI * 2.0;
        let lo_n = (self.lo / pi2).floor();
        let hi_n = (self.hi / pi2).floor();
        if (hi_n - lo_n) >= 1.0 {
            return Interval::new(-1.0, 1.0);
        }
        let lo_t = self.lo - lo_n * pi2;
        let hi_t = self.hi - hi_n * pi2;
        let (s_lo, s_hi) = if lo_n == hi_n {
            let v1 = lo_t.sin();
            let v2 = hi_t.sin();
            (v1.min(v2), v1.max(v2))
        } else {
            let mid_lo = self.lo.sin();
            let mid_hi = self.hi.sin();
            let bounds = [mid_lo, mid_hi, -1.0, 1.0];
            (
                bounds.iter().cloned().fold(f64::INFINITY, f64::min),
                bounds.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        Interval::new(s_lo, s_hi)
    }

    pub fn cos(&self) -> Interval {
        let shifted = Interval::new(
            self.lo + std::f64::consts::FRAC_PI_2,
            self.hi + std::f64::consts::FRAC_PI_2,
        );
        shifted.sin()
    }

    pub fn sqrt(&self) -> Interval {
        let lo = if self.lo < 0.0 { 0.0 } else { self.lo.sqrt() };
        let hi = if self.hi < 0.0 { 0.0 } else { self.hi.sqrt() };
        Interval::new(lo, hi)
    }

    pub fn pow(&self, exp: i32) -> Interval {
        if exp == 0 {
            return Interval::singleton(1.0);
        }
        if exp % 2 == 0 {
            let a = self.lo.abs();
            let b = self.hi.abs();
            let max_abs = a.max(b);
            let min_abs = if self.contains(0.0) { 0.0 } else { a.min(b) };
            Interval::new(min_abs.powi(exp), max_abs.powi(exp))
        } else {
            Interval::new(self.lo.powi(exp), self.hi.powi(exp))
        }
    }

    pub fn atan2(&self, other: &Interval) -> Interval {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for &x in &[self.lo, self.hi] {
            for &y in &[other.lo, other.hi] {
                let v = x.atan2(y);
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        Interval::new(lo, hi)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AbstractDomain {
    Interval(Interval),
    Bottom,
    Top,
}
