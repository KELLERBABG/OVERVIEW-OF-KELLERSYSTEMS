from typing import Dict, Tuple
import numpy as np

FREE_MUON_LIFETIME_NS = 2196.98

CAPTURE_LIFETIMES: Dict[str, float] = {
    "Hydrogen": 2197.0,
    "Carbon": 2026.0,
    "Oxygen": 1795.0,
    "Silicon": 756.0,
    "Aluminum": 864.0,
    "Iron": 206.0,
    "Copper": 163.0,
    "Lead": 75.4,
    "Uranium": 50.0,
}

def effective_muon_lifetime(material_name: str, charge: int = -1) -> float:
    if charge == 1:
        return FREE_MUON_LIFETIME_NS
    return CAPTURE_LIFETIMES.get(material_name, FREE_MUON_LIFETIME_NS)

def simulate_muon_decay_spectrum(
    material_name: str,
    num_stopped: int = 1000,
    time_bin_width_ns: float = 10.0,
    max_time_ns: float = 5000.0,
    background_rate_hz: float = 100.0,
    rng: np.random.Generator = None,
) -> Tuple[np.ndarray, np.ndarray]:
    if rng is None:
        rng = np.random.default_rng()

    tau_eff = effective_muon_lifetime(material_name, charge=-1)
    decay_times = rng.exponential(scale=tau_eff, size=num_stopped)

    bins = np.arange(0.0, max_time_ns + time_bin_width_ns, time_bin_width_ns)
    counts, bin_edges = np.histogram(decay_times, bins=bins)
    bin_centers = 0.5 * (bin_edges[:-1] + bin_edges[1:])

    bg_per_bin = background_rate_hz * (time_bin_width_ns * 1e-9) * num_stopped
    noise = rng.poisson(bg_per_bin, size=len(counts))

    return bin_centers, counts + noise
