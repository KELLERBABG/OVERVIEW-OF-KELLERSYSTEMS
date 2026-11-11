"""FAO-56 Soil Water Balance and Agronomic Calculations."""
from dataclasses import dataclass
from typing import Dict, Any, Optional
import yaml
import pathlib

# Load reference datasets
BASE_DIR = pathlib.Path(__file__).resolve().parent.parent.parent
CONFIG_DIR = BASE_DIR / 'config'

def load_yaml(filename: str) -> Dict[str, Any]:
    file_path = CONFIG_DIR / filename
    if not file_path.exists():
        return {}
    with open(file_path, 'r', encoding='utf-8') as f:
        return yaml.safe_load(f) or {}

CROPS_CONFIG = load_yaml('crops.yaml')
SOILS_CONFIG = load_yaml('soils.yaml')

@dataclass
class SoilWaterStatus:
    field_capacity_pct: float
    wilting_point_pct: float
    saturation_pct: float
    current_moisture_pct: float
    root_depth_m: float
    depletion_fraction_p: float
    taw_mm: float  # Total Available Water (mm)
    raw_mm: float  # Readily Available Water (mm)
    threshold_moisture_pct: float  # Critical VWC % below which stress begins
    depletion_mm: float  # Current root zone water deficit (mm)
    stress_index: float  # 0.0 (no stress) to 1.0 (severe wilting)
    is_stressed: bool


class WaterBalanceEngine:
    """Calculates soil water balance, crop evapotranspiration, and deficit."""

    # Root-zone weighting for multi-depth probes: shallow layers carry less of the
    # effective root zone than the middle layer.
    ROOT_ZONE_WEIGHTS = {15: 0.25, 30: 0.50, 60: 0.25}

    @classmethod
    def compute_root_zone_moisture(
        cls,
        moisture_15cm_pct: Optional[float] = None,
        moisture_30cm_pct: Optional[float] = None,
        moisture_60cm_pct: Optional[float] = None,
        fallback_pct: Optional[float] = None
    ) -> tuple:
        """Effective root-zone VWC from a multi-depth profile.

        Weights 15 cm / 30 cm / 60 cm with 25 / 50 / 25 %. Only the depths that are
        actually reported take part; the weights are renormalised. Returns
        ``(value, source)`` where source is 'multi_depth' or 'single_depth'.
        """
        readings = [
            (15, moisture_15cm_pct),
            (30, moisture_30cm_pct),
            (60, moisture_60cm_pct)
        ]
        available = [(v, cls.ROOT_ZONE_WEIGHTS[depth]) for depth, v in readings if v is not None]
        if not available:
            if fallback_pct is None:
                return (0.0, 'unknown')
            return (round(float(fallback_pct), 2), 'single_depth')
        total_weight = sum(weight for _, weight in available)
        value = sum(v * weight for v, weight in available) / total_weight
        return (round(value, 2), 'multi_depth')

    @classmethod
    def detect_deep_percolation(
        cls,
        previous_60cm_pct: Optional[float],
        current_60cm_pct: Optional[float],
        threshold_pct: float = 2.0
    ) -> bool:
        """True when the 60 cm layer gained more than `threshold_pct` of VWC.

        A sharp rise at depth means the applied water passed the root zone instead of
        being taken up by the crop.
        """
        if previous_60cm_pct is None or current_60cm_pct is None:
            return False
        return (current_60cm_pct - previous_60cm_pct) > threshold_pct

    @classmethod
    def get_crop_kc(cls, crop_key: str, growth_stage: str = 'mid_season') -> float:
        crop_data = CROPS_CONFIG.get(crop_key, {})
        stages = crop_data.get('stages', {})
        stage_info = stages.get(growth_stage)
        if stage_info:
            return float(stage_info.get('kc', 0.75))
        return 0.75

    @classmethod
    def get_crop_params(cls, crop_key: str) -> Dict[str, Any]:
        return CROPS_CONFIG.get(crop_key, {
            'name': 'Generic Crop',
            'root_depth_m': 1.0,
            'depletion_fraction_p': 0.50
        })

    @classmethod
    def get_soil_params(cls, soil_key: str) -> Dict[str, Any]:
        return SOILS_CONFIG.get(soil_key, {
            'name': 'Generic Loam',
            'field_capacity_pct': 25.0,
            'wilting_point_pct': 12.0,
            'saturation_pct': 42.0
        })

    @classmethod
    def calculate_crop_etc(cls, et0_mm: float, crop_key: str, growth_stage: str = 'mid_season') -> float:
        """Calculate Crop Evapotranspiration ETc = Kc * ET0 (mm/day)."""
        kc = cls.get_crop_kc(crop_key, growth_stage)
        return max(0.0, et0_mm * kc)

    @classmethod
    def analyze_soil_status(
        cls,
        current_moisture_pct: float,
        crop_key: str,
        growth_stage: str,
        soil_key: str,
        depletion_fraction_override: Optional[float] = None
    ) -> SoilWaterStatus:
        """Evaluate soil water reservoir levels using FAO-56 definitions.

        `depletion_fraction_override` allows the caller to apply a regulated deficit
        irrigation (RDI) strategy: a larger depletion fraction means the crop is
        allowed to use more of the readily available water before it is stressed.
        """
        soil = cls.get_soil_params(soil_key)
        crop = cls.get_crop_params(crop_key)

        fc = float(soil.get('field_capacity_pct', 22.0))
        wp = float(soil.get('wilting_point_pct', 10.0))
        sat = float(soil.get('saturation_pct', 42.0))
        zr = float(crop.get('root_depth_m', 1.0))
        p = float(crop.get('depletion_fraction_p', 0.50))
        if depletion_fraction_override is not None:
            p = max(0.1, min(0.90, float(depletion_fraction_override)))

        # TAW = 1000 * (theta_FC - theta_WP) * Zr (in mm)
        # where theta is volumetric fraction (e.g. 0.22 - 0.10)
        taw_mm = 1000.0 * ((fc - wp) / 100.0) * zr
        raw_mm = p * taw_mm

        # Threshold volumetric water content %
        # theta_threshold = theta_FC - p * (theta_FC - theta_WP)
        threshold_moisture_pct = fc - (p * (fc - wp))

        # Current water content in root zone (mm)
        # Current depletion from Field Capacity:
        # Depletion (mm) = 1000 * (theta_FC - theta_current) / 100 * Zr
        depletion_mm = max(0.0, 1000.0 * ((fc - current_moisture_pct) / 100.0) * zr)

        is_stressed = current_moisture_pct < threshold_moisture_pct

        # Stress index: 0.0 when above threshold, scales to 1.0 at wilting point
        if current_moisture_pct >= threshold_moisture_pct:
            stress_index = 0.0
        elif current_moisture_pct <= wp:
            stress_index = 1.0
        else:
            stress_index = (threshold_moisture_pct - current_moisture_pct) / max(0.1, (threshold_moisture_pct - wp))

        return SoilWaterStatus(
            field_capacity_pct=round(fc, 1),
            wilting_point_pct=round(wp, 1),
            saturation_pct=round(sat, 1),
            current_moisture_pct=round(current_moisture_pct, 1),
            root_depth_m=round(zr, 2),
            depletion_fraction_p=round(p, 2),
            taw_mm=round(taw_mm, 1),
            raw_mm=round(raw_mm, 1),
            threshold_moisture_pct=round(threshold_moisture_pct, 1),
            depletion_mm=round(depletion_mm, 1),
            stress_index=round(stress_index, 2),
            is_stressed=is_stressed
        )

    @classmethod
    def calculate_gross_depth_mm(
        cls,
        net_depth_mm: float,
        irrigation_efficiency: float = 0.90
    ) -> float:
        """Convert a NET root-zone depth into the GROSS depth the farmer must run.

        The soil has to receive `net_depth_mm`; only `irrigation_efficiency` of the
        volume that leaves the emitters infiltrates into the root zone.
        """
        eff = irrigation_efficiency if irrigation_efficiency and irrigation_efficiency > 0 else 0.90
        return net_depth_mm / eff

    @classmethod
    def calculate_net_depth_mm(
        cls,
        gross_depth_mm: float,
        irrigation_efficiency: float = 0.90
    ) -> float:
        """Convert a GROSS depth (what the grower ran) into the NET root-zone depth."""
        eff = irrigation_efficiency if irrigation_efficiency and irrigation_efficiency > 0 else 0.90
        return gross_depth_mm * eff

    @classmethod
    def adjust_moisture_for_weather(
        cls,
        current_moisture_pct: float,
        hours_since_reading: float,
        etc_day_mm: float,
        rain_mm: float,
        root_depth_m: float,
        rain_efficiency: float = 0.9
    ) -> float:
        """Correct a probe reading for the weather since the reading was taken.

        A probe reading can be several hours old. In that time the crop has consumed
        water (ETc) and rain may have replenished it. The function returns the
        moisture the root zone is expected to have now.

        * ETc is scaled from the daily value to the elapsed hours.
        * Rain is credited with the infiltration efficiency.
        """
        if hours_since_reading <= 0.5:
            return current_moisture_pct

        elapsed_etc = etc_day_mm * (min(hours_since_reading, 48.0) / 24.0)
        effective_rain = max(0.0, rain_mm) * rain_efficiency
        net_loss_mm = elapsed_etc - effective_rain

        mm_to_vwc = 100.0 / (1000.0 * max(0.2, root_depth_m))
        return max(0.0, current_moisture_pct - (net_loss_mm * mm_to_vwc))

    @classmethod
    def calculate_irrigation_duration_min(
        cls,
        target_depth_mm: float,
        application_rate_mm_h: float
    ) -> int:
        """Convert a NET target water depth (mm) into valve run time (minutes).

        `application_rate_mm_h` is the NET application rate (see
        `FieldModel.application_rate_mm_h`), therefore the resulting duration is the
        time needed to deliver `target_depth_mm` into the root zone.
        """
        if application_rate_mm_h <= 0:
            return 0
        hours = target_depth_mm / application_rate_mm_h
        return int(round(hours * 60))
