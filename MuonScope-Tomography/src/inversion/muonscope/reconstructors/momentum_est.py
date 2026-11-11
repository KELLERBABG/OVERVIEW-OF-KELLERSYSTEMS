"""Single-sample momentum estimate from a deflection in a known absorber.

.. warning::

   **This does not measure momentum, and it cannot.** Multiple Coulomb
   scattering is a random process: a muon of momentum ``p`` crossing an absorber
   receives a deflection drawn from a Gaussian of width ``theta0(p)``. One
   sample of a Gaussian is not an estimate of its width. Inverting the Highland
   relation on a single measured angle yields a number, but that number has
   order-100% relative uncertainty, and it is biased because the inversion is
   non-linear in a noisy quantity.

   The design documents' claim that a single passive lead layer "eliminates the
   particle momentum ambiguity" is therefore not supported -- see the hardware
   readiness assessment, which records the corresponding patent claim as not
   enabled.

What to use instead
-------------------
For an unknown per-event momentum, marginalise over the known cosmic-ray
spectrum: :mod:`muonscope.physics.momentum_prior` supplies the prior, and
:mod:`muonscope.reconstructors.marginalized` the estimator. That is what the
pipeline does.

These functions are retained because they are the hardware-plausible estimator
the architecture implies, and they remain useful as a diagnostic: comparing the
marginalised solution against one that trusts a single-sample estimate shows how
much the estimate costs.
"""
import numpy as np


def estimate_momentum_from_deflection(
    deflection_angle_rad: float,
    absorber_thickness_cm: float = 0.5,
    absorber_x0_cm: float = 0.56,
) -> float:
    """First-order Highland inversion of one measured deflection.

    Returns MeV/c, clipped to 200-50000. The defaults are a 5 mm lead layer
    (``X0 = 0.56 cm``). See the module docstring for why the result should not be
    treated as a measurement.
    """
    if deflection_angle_rad <= 1e-4:
        return 10000.0

    x_over_x0 = absorber_thickness_cm / absorber_x0_cm
    momentum_mev = (13.6 / deflection_angle_rad) * np.sqrt(x_over_x0)
    return float(np.clip(momentum_mev, 200.0, 50000.0))


def calculate_scattering_density_with_momentum(
    measured_variance: float,
    momentum_mev: float,
) -> float:
    """Rescale a measured ``theta^2`` to a 3 GeV reference momentum.

    ``theta^2`` scales as ``1/(beta p)^2``, so this multiplies by
    ``(p / p_ref)^2`` to bring every event to a common momentum before they are
    combined. Inherits the uncertainty of whatever produced ``momentum_mev``.
    """
    p_ref = 3000.0
    weight = (momentum_mev / p_ref) ** 2
    return measured_variance * weight

