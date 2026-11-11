"""
Unit tests for MuonScope Physics package
"""

import numpy as np
import pytest

from muonscope.physics.gaisser import gaisser_flux, sample_cosmic_muons
from muonscope.physics.scattering import highland_rms_angle, sample_moliere_scattering
from muonscope.physics.energy_loss import bethe_bloch_stopping_power
from muonscope.physics.capture import effective_muon_lifetime


def test_gaisser_flux_positive():
    energies = np.array([1.0, 3.0, 10.0, 100.0])
    zeniths = np.array([0.0, 0.5, 1.0, 1.2])
    flux = gaisser_flux(energies, zeniths)
    assert np.all(flux > 0.0)
    assert flux[0] > flux[-1]  # Flux decreases with energy


def test_sample_cosmic_muons():
    n_events = 500
    energies, directions, charges = sample_cosmic_muons(n_events, min_energy_gev=1.0)
    assert len(energies) == n_events
    assert directions.shape == (n_events, 3)
    assert np.all(directions[:, 2] < 0.0)  # Downward pointing
    # Unit vectors
    norms = np.linalg.norm(directions, axis=1)
    np.testing.assert_allclose(norms, 1.0, rtol=1e-5)
    # Charges +1 or -1
    assert set(np.unique(charges)).issubset({-1, 1})


def test_highland_scattering():
    # Concrete X0 ~ 11.55 cm, Lead X0 ~ 0.56 cm
    p_mev = 3000.0  # 3 GeV
    x_cm = 10.0

    theta_concrete = highland_rms_angle(p_mev, x_cm, radiation_length_cm=11.55)
    theta_lead = highland_rms_angle(p_mev, x_cm, radiation_length_cm=0.56)

    # Lead scatters much more strongly than concrete
    assert theta_lead > theta_concrete
    assert theta_concrete > 0.0


def test_bethe_bloch():
    # 3 GeV muon in concrete (Z=11.5, A=23, rho=2.35)
    dedx = bethe_bloch_stopping_power(3000.0, z_material=11.5, a_material=23.0, density_g_cm3=2.35)
    # Expected ~ 4 MeV/cm
    assert 2.0 < dedx < 8.0


def test_muon_capture_lifetimes():
    assert effective_muon_lifetime("Lead", charge=-1) == 75.4
    assert effective_muon_lifetime("Lead", charge=1) == 2196.98  # mu+ decays freely
