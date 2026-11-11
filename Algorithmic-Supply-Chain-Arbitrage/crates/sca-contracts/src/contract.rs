//! Settlement contract types — the core escrow agreement.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current status of a settlement contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractStatus {
    /// Draft — being negotiated.
    Draft,
    /// Funded — escrow holds the payment.
    Funded,
    /// In Transit — goods are en route.
    InTransit,
    /// Ready for settlement — delivery conditions being checked.
    ReadyForSettlement,
    /// Settled — payment released to carrier.
    Settled,
    /// Disputed — conditions not met.
    Disputed,
    /// Refunded — payment returned to sender.
    Refunded,
}

/// The type of contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractType {
    /// Simple shipment: pay on delivery.
    Shipment,
    /// Perishable shipment: pay on delivery + perishability proof.
    PerishableShipment,
    /// Multi-hop: pay per leg as each completes.
    MultiHop,
}

/// A settlement contract between a shipper and a carrier.
///
/// Funds are locked in escrow until delivery conditions are
/// proven satisfied via the custody chain and SMT verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementContract {
    /// Unique contract ID.
    pub id: Uuid,
    /// Contract type.
    pub contract_type: ContractType,
    /// Shipper (payer) identity.
    pub shipper_id: Uuid,
    /// Carrier (payee) identity.
    pub carrier_id: Uuid,
    /// Related payload/shipment ID.
    pub payload_id: Uuid,
    /// Total payment amount (USD).
    pub amount: f64,
    /// Current status.
    pub status: ContractStatus,
    /// Amount held in escrow.
    pub escrow_balance: f64,
    /// Amount released so far.
    pub released_amount: f64,
    /// Creation timestamp.
    pub created_at: i64,
    /// Last status change timestamp.
    pub updated_at: i64,
    /// Human-readable description.
    pub description: String,
}

impl SettlementContract {
    /// Create a new settlement contract.
    pub fn new(
        contract_type: ContractType,
        shipper_id: Uuid,
        carrier_id: Uuid,
        payload_id: Uuid,
        amount: f64,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            contract_type,
            shipper_id,
            carrier_id,
            payload_id,
            amount,
            status: ContractStatus::Draft,
            escrow_balance: 0.0,
            released_amount: 0.0,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            description: description.into(),
        }
    }

    /// Fund the escrow account.
    pub fn fund(&mut self, amount: f64) -> Result<(), String> {
        if self.status != ContractStatus::Draft {
            return Err("Can only fund a draft contract".into());
        }
        if amount < self.amount {
            return Err(format!("Insufficient funding: got {amount}, need {}", self.amount));
        }
        self.escrow_balance = amount;
        self.status = ContractStatus::Funded;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Mark the contract as in transit.
    pub fn start_transit(&mut self) -> Result<(), String> {
        if self.status != ContractStatus::Funded {
            return Err("Contract must be funded before transit".into());
        }
        self.status = ContractStatus::InTransit;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Settle the contract — release payment to the carrier.
    pub fn settle(&mut self) -> Result<f64, String> {
        if self.status != ContractStatus::ReadyForSettlement {
            return Err("Contract not ready for settlement".into());
        }
        let payout = self.escrow_balance;
        self.released_amount = payout;
        self.escrow_balance = 0.0;
        self.status = ContractStatus::Settled;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(payout)
    }

    /// Flag the contract as disputed.
    pub fn dispute(&mut self, _reason: &str) {
        self.status = ContractStatus::Disputed;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Refund the escrow to the shipper.
    pub fn refund(&mut self) -> Result<f64, String> {
        if self.status != ContractStatus::Disputed {
            return Err("Only disputed contracts can be refunded".into());
        }
        let refund = self.escrow_balance;
        self.escrow_balance = 0.0;
        self.status = ContractStatus::Refunded;
        self.updated_at = chrono::Utc::now().timestamp();
        Ok(refund)
    }

    /// Mark as ready for settlement after proof verification.
    pub fn mark_ready(&mut self) {
        self.status = ContractStatus::ReadyForSettlement;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_lifecycle() {
        let mut contract = SettlementContract::new(
            ContractType::Shipment,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            5000.0,
            "Shipment of wheat",
        );

        assert_eq!(contract.status, ContractStatus::Draft);

        contract.fund(5000.0).unwrap();
        assert_eq!(contract.status, ContractStatus::Funded);

        contract.start_transit().unwrap();
        assert_eq!(contract.status, ContractStatus::InTransit);

        contract.mark_ready();
        assert_eq!(contract.status, ContractStatus::ReadyForSettlement);

        let payout = contract.settle().unwrap();
        assert_eq!(payout, 5000.0);
        assert_eq!(contract.status, ContractStatus::Settled);
    }

    #[test]
    fn test_insufficient_funding() {
        let mut contract = SettlementContract::new(
            ContractType::Shipment,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            5000.0,
            "Test",
        );
        assert!(contract.fund(3000.0).is_err());
    }

    #[test]
    fn test_dispute_and_refund() {
        let mut contract = SettlementContract::new(
            ContractType::Shipment,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            5000.0,
            "Test",
        );
        contract.fund(5000.0).unwrap();
        contract.dispute("Cold chain broken");
        assert_eq!(contract.status, ContractStatus::Disputed);

        let refund = contract.refund().unwrap();
        assert_eq!(refund, 5000.0);
        assert_eq!(contract.status, ContractStatus::Refunded);
    }
}
