use crate::field::ScalarField;

#[derive(Debug, Clone, PartialEq)]
pub struct Share {
    pub index: u64,
    pub value: ScalarField,
}

impl Share {
    pub fn new(index: u64, value: ScalarField) -> Self {
        Self { index, value }
    }

    pub fn x(&self) -> ScalarField {
        ScalarField::from(self.index)
    }

    pub fn as_pair(&self) -> (ScalarField, ScalarField) {
        (self.x(), self.value)
    }
}
