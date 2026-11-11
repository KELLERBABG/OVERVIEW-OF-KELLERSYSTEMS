use mesh_credit::constraints::{ConstraintEngine, Exposure};
use mesh_credit::loan::{calculate_interest_rate, Loan, LoanStatus};
use mesh_graph_core::graph::SocialGraph;
use mesh_smpc::solvency::{verify_solvency, PartyShare};
use mesh_zkp::proof::ClusterMembershipProof;
use serde::{Deserialize, Serialize};

use crate::pool::CreditPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolLoanRequest {
    pub borrower_id: String,
    pub amount: f64,
    pub purpose: String,
    pub solvency_shares: Option<Vec<PartyShare>>,
    pub cluster_proof: Option<ClusterMembershipProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolLoanResult {
    pub loan_id: String,
    pub approved: bool,
    pub reason: String,
    pub interest_rate: f64,
    pub principal: f64,
}

pub struct SettlementEngine;

impl SettlementEngine {
    pub fn evaluate_pool_loan(
        pool: &mut CreditPool,
        request: &PoolLoanRequest,
        graph: &SocialGraph,
        existing_exposures: &[Exposure],
        smpc_threshold: u8,
        min_cluster_size: usize,
        min_reputation: f64,
    ) -> PoolLoanResult {
        if pool.available_liquidity() < request.amount {
            return PoolLoanResult {
                loan_id: String::new(),
                approved: false,
                reason: format!(
                    "insufficient pool liquidity: available {:.2}, requested {:.2}",
                    pool.available_liquidity(),
                    request.amount
                ),
                interest_rate: 0.0,
                principal: 0.0,
            };
        }

        match ConstraintEngine::check_borrower(
            graph,
            &request.borrower_id,
            request.amount,
            existing_exposures,
        ) {
            Ok((false, _)) => {
                return PoolLoanResult {
                    loan_id: String::new(),
                    approved: false,
                    reason: "borrower does not meet credit constraints".to_string(),
                    interest_rate: 0.0,
                    principal: 0.0,
                };
            }
            Err(e) => {
                return PoolLoanResult {
                    loan_id: String::new(),
                    approved: false,
                    reason: format!("constraint error: {}", e),
                    interest_rate: 0.0,
                    principal: 0.0,
                };
            }
            Ok((true, _)) => {}
        }

        if let Some(ref shares) = request.solvency_shares {
            match verify_solvency(&request.borrower_id, shares, smpc_threshold) {
                Ok(result) => {
                    if !result.solvent {
                        return PoolLoanResult {
                            loan_id: String::new(),
                            approved: false,
                            reason: format!(
                                "SMPC solvency check failed: net position {:.2}",
                                result.net_position
                            ),
                            interest_rate: 0.0,
                            principal: 0.0,
                        };
                    }
                }
                Err(e) => {
                    return PoolLoanResult {
                        loan_id: String::new(),
                        approved: false,
                        reason: format!("SMPC verification error: {}", e),
                        interest_rate: 0.0,
                        principal: 0.0,
                    };
                }
            }
        }

        if let Some(ref proof) = request.cluster_proof {
            match proof.verify(min_cluster_size, min_reputation) {
                Ok(valid) => {
                    if !valid {
                        return PoolLoanResult {
                            loan_id: String::new(),
                            approved: false,
                            reason: format!(
                                "zk-cluster proof invalid: need size>={} rep>={}, got size={} rep={:.4}",
                                min_cluster_size, min_reputation, proof.cluster_size, proof.avg_reputation
                            ),
                            interest_rate: 0.0,
                            principal: 0.0,
                        };
                    }
                }
                Err(e) => {
                    return PoolLoanResult {
                        loan_id: String::new(),
                        approved: false,
                        reason: format!("zk-proof verification error: {}", e),
                        interest_rate: 0.0,
                        principal: 0.0,
                    };
                }
            }
        }

        let base_rate = pool.effective_apr();
        let rate = calculate_interest_rate(graph, &request.borrower_id, base_rate);

        if let Err(e) = pool.allocate(request.amount) {
            return PoolLoanResult {
                loan_id: String::new(),
                approved: false,
                reason: format!("allocation failed: {}", e),
                interest_rate: 0.0,
                principal: 0.0,
            };
        }

        let loan_id = format!("pool_loan_{}", chrono_now());

        PoolLoanResult {
            loan_id,
            approved: true,
            reason: "all constraints passed".to_string(),
            interest_rate: rate,
            principal: request.amount,
        }
    }

    pub fn process_repayment(
        pool: &mut CreditPool,
        loans: &mut [Loan],
        loan_id: &str,
        amount: f64,
    ) -> Result<f64, String> {
        let loan = loans
            .iter_mut()
            .find(|l| l.id == loan_id)
            .ok_or_else(|| format!("loan '{}' not found", loan_id))?;

        if loan.status != LoanStatus::Active {
            return Err("loan is not active".to_string());
        }

        let repaid = loan.repay(amount)?;
        pool.repay_allocation(repaid);

        Ok(repaid)
    }

    pub fn auto_issue_from_pool(
        pool: &mut CreditPool,
        graph: &SocialGraph,
        borrower_id: &str,
        amount: f64,
        purpose: &str,
        exposures: &[Exposure],
    ) -> PoolLoanResult {
        let request = PoolLoanRequest {
            borrower_id: borrower_id.to_string(),
            amount,
            purpose: purpose.to_string(),
            solvency_shares: None,
            cluster_proof: None,
        };

        Self::evaluate_pool_loan(pool, &request, graph, exposures, 3, 3, 0.1)
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
