"""Stage 2: Photochemical Reaction Kinetics & Oxygen Scavenging Simulation.

Simulates the non-linear threshold dynamics of free-radical photopolymerization:
1. Oxygen scavenging rate: d[O2]/dt = - k_inh * [R•] * [O2]
2. Radical generation rate: d[R•]/dt = 2.0 * phi_i * eps * [PI] * I(t) / (h * nu) - k_inh * [R•] * [O2] - 2 * k_t * [R•]^2
3. Monomer conversion rate: d[M]/dt = - k_p * [M] * [R•]
4. Polymer conversion: chi(t) = 1.0 - [M](t) / [M]_0

Predicts induction delay (time required to deplete dissolved oxygen) and verifies
whether the full 3D solid crosslinks in the 15-second exposure window.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np


@dataclass
class KineticParameters:
    """Reaction rate constants for 405nm photopolymerization in PEGDA / HDDA."""
    name: str = "pegda-405"
    initial_oxygen_mM: float = 1.4       # dissolved O2 in ambient resin (~1.4 mM)
    critical_oxygen_mM: float = 0.05     # critical threshold where radical propagation dominates
    initial_monomer_M: float = 5.2       # double-bond molar concentration (~5.2 M for PEGDA)
    initial_pi_wt_percent: float = 1.0   # TPO-L photoinitiator concentration (1.0 wt%)
    quantum_yield: float = 0.55          # initiation quantum efficiency phi_i
    k_inhibition: float = 1.5e6          # radical-O2 scavenging rate (M^-1 s^-1)
    k_propagation: float = 8.5e2         # monomer chain propagation rate (M^-1 s^-1)
    k_termination: float = 2.0e4         # bimolecular radical termination rate (M^-1 s^-1)
    gel_conversion: float = 0.08         # Flory-Stockmayer gel point conversion (~8% for diacrylates)


@dataclass
class KineticSimulationResult:
    time_s: np.ndarray
    oxygen_mM: np.ndarray
    radicals_uM: np.ndarray
    conversion: np.ndarray
    induction_time_s: float
    gel_time_s: float
    gelled: bool


def simulate_kinetics(
    intensity_mW_cm2: float,
    params: KineticParameters | None = None,
    total_time_s: float = 15.0,
    n_steps: int = 1500,
) -> KineticSimulationResult:
    """Solve transient photopolymerization ODEs for a given optical intensity.
    
    Parameters
    ----------
    intensity_mW_cm2 : float
        Irradiance at the voxel (mW/cm^2).
    params : KineticParameters, optional
        Reaction rate parameters.
    total_time_s : float
        Exposure window in seconds (default 15.0 s).
    n_steps : int
        Time discretization steps.
    """
    params = params or KineticParameters()
    dt = total_time_s / n_steps
    times = np.linspace(0.0, total_time_s, n_steps + 1)
    
    # State variables
    O2 = float(params.initial_oxygen_mM * 1e-3)  # mol/L
    M = float(params.initial_monomer_M)          # mol/L
    M0 = M
    R = 0.0                                      # mol/L
    
    # Optical initiation rate R_i = 2 * phi_i * I_absorbed
    # At 405nm, photon energy E_photon = h * c / lambda = 4.90e-19 J = 2.95e5 J/mol
    # Intensity I (W/m^2 = 10^-3 W/cm^2 -> 10 W/m^2 per mW/cm^2)
    # R_initiation ~ alpha * I / E_photon
    I_W_cm2 = intensity_mW_cm2 * 1e-3
    # Approximate initiation radical rate (mol / (L * s))
    R_i = (params.quantum_yield * I_W_cm2 * 0.01) / max(0.005, 1e-6)
    
    o2_history = np.zeros(n_steps + 1)
    r_history = np.zeros(n_steps + 1)
    conv_history = np.zeros(n_steps + 1)
    
    o2_history[0] = O2 * 1e3
    r_history[0] = R * 1e6
    conv_history[0] = 0.0
    
    induction_time = float("nan")
    gel_time = float("nan")
    
    for i in range(1, n_steps + 1):
        t = times[i]
        
        # 1. Oxygen consumption
        dO2_dt = - params.k_inhibition * R * O2
        # 2. Radical balance
        dR_dt = R_i - params.k_inhibition * R * O2 - 2.0 * params.k_termination * (R ** 2)
        # 3. Monomer propagation (only significant once O2 is substantially depleted)
        # Oxygen quenching factor
        o2_inhibition_factor = max(0.0, 1.0 - (O2 / (params.initial_oxygen_mM * 1e-3 + 1e-12)))
        dM_dt = - params.k_propagation * M * R * (o2_inhibition_factor ** 2)
        
        # Forward Euler integration with positive bounds
        O2 = max(0.0, O2 + dO2_dt * dt)
        R = max(0.0, R + dR_dt * dt)
        M = max(0.0, min(M0, M + dM_dt * dt))
        
        conv = 1.0 - (M / M0)
        
        o2_history[i] = O2 * 1e3
        r_history[i] = R * 1e6
        conv_history[i] = conv
        
        # Check induction threshold
        if np.isnan(induction_time) and O2 <= (params.critical_oxygen_mM * 1e-3):
            induction_time = t
            
        # Check gel threshold
        if np.isnan(gel_time) and conv >= params.gel_conversion:
            gel_time = t

    return KineticSimulationResult(
        time_s=times,
        oxygen_mM=o2_history,
        radicals_uM=r_history,
        conversion=conv_history,
        induction_time_s=float(induction_time) if not np.isnan(induction_time) else total_time_s,
        gel_time_s=float(gel_time) if not np.isnan(gel_time) else total_time_s,
        gelled=bool(not np.isnan(gel_time) and gel_time <= total_time_s),
    )
