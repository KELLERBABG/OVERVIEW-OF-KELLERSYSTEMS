use ark_bn254::Fr;
use sss::Share;

pub type Scalar = Fr;

#[derive(Debug, Clone, PartialEq)]
pub struct SecretShare {
    pub share: Share,
}

impl SecretShare {
    pub fn new(share: Share) -> Self {
        Self { share }
    }

    pub fn value(&self) -> &Scalar {
        &self.share.value
    }

    pub fn index(&self) -> u64 {
        self.share.index
    }
}
