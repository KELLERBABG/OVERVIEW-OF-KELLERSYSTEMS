#!/usr/bin/env python3
"""
Quickstart example: Generate synthetic data with differential privacy.

Usage:
    python examples/quickstart.py

This example:
1. Creates a sample dataset with mixed types
2. Trains a CTGAN-DP model with ε=10.0
3. Generates synthetic data
4. Evaluates utility and privacy
"""

import numpy as np
from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig
from dpsynth.evaluation.utility import UtilityEvaluator
from dpsynth.evaluation.privacy import MembershipInferenceAttack

def main():
    print("=" * 60)
    print("dpsynth Quickstart — DP Synthetic Data Generation")
    print("=" * 60)

    # ── Generate sample data ──────────────────────────────────────
    print("\n1. Creating sample dataset...")
    rng = np.random.RandomState(42)
    n = 2000
    age = rng.randint(18, 70, n)
    income = 30000 + age * 500 + rng.randn(n) * 10000
    education = rng.choice(["HS", "BS", "MS", "PhD"], n, p=[0.3, 0.4, 0.2, 0.1])
    category = rng.choice(["A", "B", "C"], n, p=[0.5, 0.3, 0.2])

    real_data = np.column_stack([age, income, education, category])
    real_data = real_data.astype(object)
    print(f"   Shape: {real_data.shape}")
    print(f"   Types: age(int), income(float), education(str), category(str)")

    # ── Configure and train ───────────────────────────────────────
    print("\n2. Training CTGAN-DP model...")
    config = CTGANConfig(
        epsilon=10.0,
        delta=1e-5,
        epochs=20,
        batch_size=200,
        latent_dim=64,
        max_grad_norm=1.0,
        device="cpu",
        verbose=True,
    )
    model = CTGAN_DP(config)
    budget = model.fit(real_data)
    print(f"   Privacy spent: ε = {budget.epsilon:.2f}, δ = {budget.delta:.1e}")

    # ── Generate synthetic data ───────────────────────────────────
    print("\n3. Generating synthetic data...")
    synthetic = model.generate(5000)
    print(f"   Generated {len(synthetic)} samples, shape: {synthetic.shape}")

    # ── Evaluate utility ──────────────────────────────────────────
    print("\n4. Evaluating utility...")
    evaluator = UtilityEvaluator()
    report = evaluator.evaluate(real_data, synthetic)
    print(f"   Overall utility: {report.overall_score:.2%}")
    print(f"   Column similarities: {report.column_wise_similarity}")
    print(f"   Correlation distance: {report.correlation_distance:.4f}")

    # ── Audit privacy ─────────────────────────────────────────────
    print("\n5. Auditing privacy (Membership Inference)...")
    mia = MembershipInferenceAttack(n_shadow=10)
    privacy_report = mia.evaluate(real_data, synthetic)
    print(f"   Attack AUC: {privacy_report.attack_auc:.3f}")
    if privacy_report.is_protected:
        print("   ✓ Data is protected against membership inference")
    else:
        print("   ⚠ Data may have privacy leakage — consider lower ε")

    print("\n" + "=" * 60)
    print("Quickstart complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()