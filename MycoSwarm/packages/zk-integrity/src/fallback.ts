/**
 * Probabilistic verification strategy:
 * Only require zk-proofs for a random subset of work units.
 *
 * If a node cheats, it has a 1/N chance of being caught.
 * With N=20, a rational node shouldn't cheat (expected loss > gain).
 */
export class ProbabilisticVerification {
  private verificationProbability: number;

  constructor(probability: number = 0.05) {
    this.verificationProbability = probability;
  }

  shouldVerify(): boolean {
    return Math.random() < this.verificationProbability;
  }

  /**
   * If we suspect cheating, increase verification probability
   * for that node's future units (Bayesian update).
   */
  adjustForSuspicion(_nodeId: string, previousViolations: number): void {
    if (previousViolations > 0) {
      this.verificationProbability = Math.min(1, this.verificationProbability * 2);
    }
  }

  getProbability(): number {
    return this.verificationProbability;
  }

  reset(): void {
    this.verificationProbability = 0.05;
  }
}