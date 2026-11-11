"""Entry point for `python -m dpsynth`.

Provides a quick overview of the package and its version.
"""

from dpsynth import __version__


def main() -> None:
    """Print package info when run as `python -m dpsynth`."""
    print(f"dpsynth v{__version__}")
    print("Differentially Private Synthetic Data Generator")
    print()
    print("Modules:")
    print("  dpsynth.privacy      — Privacy accounting, clipping, noise, exponential mechanism")
    print("  dpsynth.models       — DP model architectures (CTGAN-DP)")
    print("  dpsynth.evaluation   — Utility metrics, MIA, post-processing verification")
    print("  dpsynth.utils        — Dataset loaders and data preprocessing")
    print()
    print("Quick start:")
    print("  from dpsynth.models.ctgan_dp import CTGAN_DP, CTGANConfig")
    print("  model = CTGAN_DP(CTGANConfig(epsilon=10.0))")
    print("  model.fit(data)")
    print("  synthetic = model.generate(1000)")
    print()
    print("See docs/ for tutorials and the API reference.")


if __name__ == "__main__":
    main()
