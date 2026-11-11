use serde::{Deserialize, Serialize};

pub const FIELD_PRIME: u64 = 2147483647;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fp(u64);

impl Fp {
    pub fn new(value: u64) -> Self {
        Fp(value % FIELD_PRIME)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn zero() -> Self {
        Fp(0)
    }

    pub fn one() -> Self {
        Fp(1)
    }

    pub fn inv(self) -> Fp {
        self.pow(FIELD_PRIME - 2)
    }

    pub fn pow(self, exp: u64) -> Fp {
        let mut result = Fp::one();
        let mut base = self;
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            e >>= 1;
        }
        result
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Fp::new(rng.gen::<u64>())
    }
}

impl std::ops::Add for Fp {
    type Output = Fp;
    fn add(self, other: Fp) -> Fp {
        let sum = self.0 + other.0;
        Fp(if sum >= FIELD_PRIME {
            sum - FIELD_PRIME
        } else {
            sum
        })
    }
}

impl std::ops::Sub for Fp {
    type Output = Fp;
    fn sub(self, other: Fp) -> Fp {
        let (val, overflow) = self.0.overflowing_sub(other.0);
        Fp(if overflow {
            val.wrapping_add(FIELD_PRIME)
        } else {
            val
        })
    }
}

impl std::ops::Mul for Fp {
    type Output = Fp;
    fn mul(self, other: Fp) -> Fp {
        Fp((self.0 as u128 * other.0 as u128 % FIELD_PRIME as u128) as u64)
    }
}

impl std::ops::Div for Fp {
    type Output = Fp;
    fn div(self, other: Fp) -> Fp {
        std::ops::Mul::mul(self, other.inv())
    }
}

impl std::ops::Neg for Fp {
    type Output = Fp;
    fn neg(self) -> Fp {
        Fp(if self.0 == 0 { 0 } else { FIELD_PRIME - self.0 })
    }
}

impl From<u64> for Fp {
    fn from(v: u64) -> Self {
        Fp::new(v)
    }
}

impl std::fmt::Display for Fp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
