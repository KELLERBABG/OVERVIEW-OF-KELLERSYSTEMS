"""
Physics subpackage: Cosmic ray flux spectra, Multiple Coulomb Scattering,
continuous energy loss, and bound negative muon capture kinetics.
"""

from .gaisser import (
    gaisser_flux,
    sample_cosmic_muons,
)
from .scattering import (
    highland_rms_angle,
    sample_moliere_scattering,
    Material,
)
from .energy_loss import (
    bethe_bloch_stopping_power,
    muon_survival_probability,
)
from .capture import (
    effective_muon_lifetime,
    simulate_muon_decay_spectrum,
)

__all__ = [
    "gaisser_flux",
    "sample_cosmic_muons",
    "highland_rms_angle",
    "sample_moliere_scattering",
    "Material",
    "bethe_bloch_stopping_power",
    "muon_survival_probability",
    "effective_muon_lifetime",
    "simulate_muon_decay_spectrum",
]
