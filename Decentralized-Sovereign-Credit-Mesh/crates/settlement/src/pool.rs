use mesh_credit::money::validate_amount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityProvider {
    pub id: String,
    pub deposited: f64,
    pub shares: f64,
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditPool {
    pub id: String,
    pub name: String,
    pub total_liquidity: f64,
    pub allocated: f64,
    pub target_apr: f64,
    pub providers: Vec<LiquidityProvider>,
    pub created_at: i64,
    pub total_shares: f64,
}

impl CreditPool {
    pub fn new(id: String, name: String, target_apr: f64) -> Self {
        CreditPool {
            id,
            name,
            total_liquidity: 0.0,
            allocated: 0.0,
            target_apr,
            providers: Vec::new(),
            created_at: chrono_now(),
            total_shares: 0.0,
        }
    }

    pub fn available_liquidity(&self) -> f64 {
        self.total_liquidity - self.allocated
    }

    pub fn deposit(&mut self, provider_id: &str, amount: f64) -> Result<f64, String> {
        validate_amount("deposit amount", amount)?;

        let shares = if self.total_shares > 0.0 {
            (amount / self.total_liquidity) * self.total_shares
        } else {
            amount
        };

        if let Some(provider) = self.providers.iter_mut().find(|p| p.id == provider_id) {
            provider.deposited += amount;
            provider.shares += shares;
        } else {
            self.providers.push(LiquidityProvider {
                id: provider_id.to_string(),
                deposited: amount,
                shares,
                joined_at: chrono_now(),
            });
        }

        self.total_liquidity += amount;
        self.total_shares += shares;
        Ok(shares)
    }

    pub fn withdraw(&mut self, provider_id: &str, amount: f64) -> Result<f64, String> {
        validate_amount("withdraw amount", amount)?;

        let available = self.available_liquidity();
        let total_shares = self.total_shares;
        let total_liquidity = self.total_liquidity;

        let (from_pool, shares_to_redeem) = {
            let provider = self
                .providers
                .iter_mut()
                .find(|p| p.id == provider_id)
                .ok_or_else(|| format!("provider '{}' not found", provider_id))?;

            let actual = amount.min(provider.deposited);
            let from_pool = actual.min(available);

            let shares_to_redeem = if total_shares > 0.0 {
                (from_pool / total_liquidity) * total_shares
            } else {
                from_pool
            };

            provider.deposited -= from_pool;
            provider.shares -= shares_to_redeem;
            (from_pool, shares_to_redeem)
        };

        self.total_liquidity -= from_pool;
        self.total_shares -= shares_to_redeem;

        Ok(from_pool)
    }

    pub fn allocate(&mut self, amount: f64) -> Result<(), String> {
        validate_amount("allocation", amount)?;
        if amount > self.available_liquidity() {
            return Err(format!(
                "insufficient liquidity: have {:.2}, need {:.2}",
                self.available_liquidity(),
                amount
            ));
        }
        self.allocated += amount;
        Ok(())
    }

    pub fn repay_allocation(&mut self, amount: f64) {
        let repaid = amount.min(self.allocated);
        self.allocated -= repaid;
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn utilization_rate(&self) -> f64 {
        if self.total_liquidity > 0.0 {
            self.allocated / self.total_liquidity
        } else {
            0.0
        }
    }

    pub fn effective_apr(&self) -> f64 {
        self.target_apr * (1.0 + self.utilization_rate())
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
