//! Escrow account management.

use uuid::Uuid;

/// An escrow account holding funds for a settlement contract.
#[derive(Debug, Clone)]
pub struct EscrowAccount {
    /// Account ID.
    pub id: Uuid,
    /// Contract ID this escrow belongs to.
    pub contract_id: Uuid,
    /// Current balance.
    pub balance: f64,
    /// Total deposited over lifetime.
    pub total_deposited: f64,
    /// Total withdrawn over lifetime.
    pub total_withdrawn: f64,
}

impl EscrowAccount {
    pub fn new(contract_id: Uuid) -> Self {
        Self { id: Uuid::new_v4(), contract_id, balance: 0.0, total_deposited: 0.0, total_withdrawn: 0.0 }
    }

    /// Deposit funds into escrow.
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
        self.total_deposited += amount;
    }

    /// Withdraw funds from escrow.
    pub fn withdraw(&mut self, amount: f64) -> Result<f64, String> {
        let actual = amount.min(self.balance);
        self.balance -= actual;
        self.total_withdrawn += actual;
        Ok(actual)
    }

    pub fn is_empty(&self) -> bool {
        self.balance <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_withdraw() {
        let mut escrow = EscrowAccount::new(Uuid::new_v4());
        escrow.deposit(1000.0);
        assert_eq!(escrow.balance, 1000.0);

        let withdrawn = escrow.withdraw(600.0).unwrap();
        assert_eq!(withdrawn, 600.0);
        assert_eq!(escrow.balance, 400.0);
    }
}
