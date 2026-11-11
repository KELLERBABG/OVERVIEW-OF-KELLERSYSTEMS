import numpy as np

ELECTRON_MASS_MEV = 0.51099895
MUON_MASS_MEV = 105.6583755
K_CONSTANT = 0.307075

def bethe_bloch_stopping_power(
    kinetic_energy_mev: float,
    z_material: float,
    a_material: float,
    density_g_cm3: float,
    mean_excitation_ev: float = 136.0,
) -> float:
    if kinetic_energy_mev <= 0.1:
        return 100.0

    total_energy = kinetic_energy_mev + MUON_MASS_MEV
    gamma = total_energy / MUON_MASS_MEV
    beta = np.sqrt(max(1.0 - 1.0 / (gamma**2), 1e-6))
    
    s = ELECTRON_MASS_MEV / MUON_MASS_MEV
    w_max = (2.0 * ELECTRON_MASS_MEV * (gamma**2 - 1.0)) / (1.0 + 2.0 * gamma * s + s**2)

    i_mev = mean_excitation_ev * 1e-6
    plasma_energy_mev = 28.816e-6 * np.sqrt(density_g_cm3 * (z_material / a_material))

    x = np.log10(max(beta * gamma, 1e-4))
    delta = max(2.0 * np.log(10.0) * x - 2.0 * np.log(i_mev / plasma_energy_mev) - 1.0, 0.0)

    term1 = K_CONSTANT * (z_material / a_material) * (1.0 / beta**2)
    term2 = 0.5 * np.log((2.0 * ELECTRON_MASS_MEV * (beta**2) * (gamma**2) * w_max) / (i_mev**2)) - beta**2 - delta / 2.0

    return float(term1 * max(term2, 0.5) * density_g_cm3)

def muon_survival_probability(
    initial_energy_gev: float,
    integrated_opacity_hg_cm2: float,
) -> float:
    min_energy_req_gev = 2.0e-3 * (integrated_opacity_hg_cm2 * 100.0)
    return 1.0 if initial_energy_gev > min_energy_req_gev else 0.0
