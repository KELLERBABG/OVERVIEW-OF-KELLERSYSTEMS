pub trait CommitmentScheme {
    type Commitment: Clone + PartialEq;
    type Opening: Clone;
    type Error;

    fn commit(&self, value: &[u8]) -> (Self::Commitment, Self::Opening);

    fn verify(
        &self,
        commitment: &Self::Commitment,
        value: &[u8],
        opening: &Self::Opening,
    ) -> Result<bool, Self::Error>;
}
