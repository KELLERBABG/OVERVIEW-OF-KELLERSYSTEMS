use mesh_graph_core::algorithms;
use mesh_graph_core::graph::SocialGraph;
use serde::{Deserialize, Serialize};

use crate::money::validate_amount;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoanStatus {
    Pending,
    Active,
    Repaid,
    Defaulted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loan {
    pub id: String,
    pub borrower: String,
    pub lender: String,
    pub principal: f64,
    pub interest_rate: f64,
    pub outstanding: f64,
    pub status: LoanStatus,
    pub created_at: i64,
    pub due_at: Option<i64>,
    pub purpose: String,
}

impl Loan {
    pub fn new(
        id: String,
        borrower: String,
        lender: String,
        principal: f64,
        interest_rate: f64,
        purpose: String,
    ) -> Result<Self, String> {
        validate_amount("principal", principal)?;
        if !interest_rate.is_finite() || interest_rate < 0.0 {
            return Err(format!(
                "interest_rate must be a finite, non-negative number, got {interest_rate}"
            ));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Ok(Loan {
            id,
            borrower,
            lender,
            principal,
            interest_rate,
            outstanding: principal,
            status: LoanStatus::Pending,
            created_at: now,
            due_at: Some(now + 30 * 86400),
            purpose,
        })
    }

    pub fn drawdown(&mut self) -> Result<(), String> {
        if self.status != LoanStatus::Pending {
            return Err("loan is not in pending status".to_string());
        }
        self.status = LoanStatus::Active;
        Ok(())
    }

    pub fn repay(&mut self, amount: f64) -> Result<f64, String> {
        if self.status != LoanStatus::Active {
            return Err("loan is not active".to_string());
        }
        validate_amount("repayment amount", amount)?;

        let repayment = amount.min(self.outstanding);
        self.outstanding -= repayment;

        if self.outstanding <= 0.0 {
            self.status = LoanStatus::Repaid;
            self.outstanding = 0.0;
        }

        Ok(repayment)
    }

    pub fn default(&mut self) -> Result<(), String> {
        if self.status != LoanStatus::Active {
            return Err("loan is not active".to_string());
        }
        self.status = LoanStatus::Defaulted;
        Ok(())
    }

    pub fn accrued_interest(&self, at_time: i64) -> f64 {
        if self.status != LoanStatus::Active {
            return 0.0;
        }
        let elapsed_days = (at_time - self.created_at).max(0) as f64 / 86400.0;
        let daily_rate = self.interest_rate / 365.0;
        self.outstanding * daily_rate * elapsed_days
    }
}

pub fn calculate_interest_rate(graph: &SocialGraph, borrower: &str, base_rate: f64) -> f64 {
    let pr = algorithms::compute_pagerank(graph, 0.85, 100, 1e-6);
    let trust_score = pr.get(borrower).copied().unwrap_or(0.0);

    let rating = if trust_score > 0.05 {
        "low_risk"
    } else if trust_score > 0.005 {
        "medium_risk"
    } else {
        "high_risk"
    };

    let spread = match rating {
        "low_risk" => 0.02,
        "medium_risk" => 0.06,
        "high_risk" => 0.12,
        _ => 0.10,
    };

    let sybil_penalty =
        mesh_graph_core::entropy::sybil_score(graph, borrower).unwrap_or(0.0) * 0.05;

    base_rate + spread + sybil_penalty
}
