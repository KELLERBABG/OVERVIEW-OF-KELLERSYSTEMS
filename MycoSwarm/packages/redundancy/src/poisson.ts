/**
 * Poisson probability mass function: P(k; λ) = (λ^k * e^-λ) / k!
 *
 * Used to model the probability that exactly k nodes will stay online
 * given an average dropout rate λ.
 */
export function poissonPMF(k: number, lambda: number): number {
  return (Math.pow(lambda, k) * Math.exp(-lambda)) / factorial(k);
}

function factorial(n: number): number {
  if (n <= 1) return 1;
  let result = 1;
  for (let i = 2; i <= n; i++) result *= i;
  return result;
}

/**
 * Compute the minimum redundancy factor `r` such that the probability
 * of at least one node completing the work is ≥ targetReliability.
 *
 * λ (lambda) = r * (1 - historical_dropout_rate)
 * We want P(at least 1 success) = 1 - e^(-λ) ≥ targetReliability
 * => r ≥ -ln(1 - targetReliability) / (1 - dropoutRate)
 */
export function minRedundancyFactor(
  historicalDropoutRate: number,
  targetReliability: number,
): number {
  if (targetReliability >= 1) return Infinity;
  const lambda = -Math.log(1 - targetReliability);
  const effectiveRate = 1 - historicalDropoutRate;
  if (effectiveRate <= 0) return Infinity;
  return Math.ceil(lambda / effectiveRate);
}