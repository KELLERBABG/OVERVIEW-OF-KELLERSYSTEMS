"""Physics-based soil moisture and sensor simulation engine."""
import math
import random
from dataclasses import dataclass
from typing import List
from datetime import datetime, timedelta, timezone
from ..core.models import FieldModel
from ..core.water_balance import WaterBalanceEngine

@dataclass
class SimulationStepResult:
    timestamp: datetime
    hour: int
    soil_moisture_pct: float
    soil_temp_c: float
    battery_volts: float
    etc_mm: float
    rain_mm: float
    irrigation_mm: float
    drainage_mm: float
    stress_index: float

class SoilSensorSimulator:
    """Simulates continuous root-zone soil water physics and telemetry generation."""

    def __init__(
        self,
        field: FieldModel,
        initial_moisture_pct: float = 24.0,
        battery_volts: float = 3.65
    ):
        self.field = field
        self.current_moisture_pct = initial_moisture_pct
        self.battery_volts = battery_volts
        self.soil_params = WaterBalanceEngine.get_soil_params(field.soil_key)
        self.crop_params = WaterBalanceEngine.get_crop_params(field.crop_key)
        
        self.fc = float(self.soil_params.get('field_capacity_pct', 22.0))
        self.wp = float(self.soil_params.get('wilting_point_pct', 10.0))
        self.sat = float(self.soil_params.get('saturation_pct', 42.0))
        self.root_depth_m = float(self.crop_params.get('root_depth_m', 1.0))

    def step_hour(
        self,
        current_time: datetime,
        et0_day_mm: float,
        air_temp_c: float,
        rain_mm: float = 0.0,
        irrigation_mm: float = 0.0
    ) -> SimulationStepResult:
        """Advance simulation by 1 hour with soil physics equations."""
        hour = current_time.hour

        # 1. Solar diurnal curve for hourly ETc fraction:
        # Solar radiation peaks around 13:00-14:00, zero at night (20:00 - 06:00)
        if 6 <= hour <= 20:
            # Half-sine bell curve over 14 daylight hours
            solar_phase = (hour - 6) / 14.0 * math.pi
            hourly_etc_frac = math.sin(solar_phase) / 9.0  # Integral sums to ~1.0
        else:
            hourly_etc_frac = 0.005 # Minimal nocturnal transpiration

        daily_etc = WaterBalanceEngine.calculate_crop_etc(et0_day_mm, self.field.crop_key, self.field.growth_stage)
        hourly_etc_mm = daily_etc * hourly_etc_frac

        # 2. Water Infiltration & Replenishment
        # 1 mm of water in root zone (Zr meters) = (1 mm / (1000 * Zr)) * 100 % VWC
        mm_to_vwc = 100.0 / (1000.0 * max(0.2, self.root_depth_m))
        
        added_water_mm = (rain_mm * 0.9) + irrigation_mm
        added_vwc = added_water_mm * mm_to_vwc

        # 3. Evapotranspirative loss
        # If soil is below wilting point, root extraction slows down (stress decay)
        ks = 1.0
        if self.current_moisture_pct < self.wp:
            ks = max(0.1, (self.current_moisture_pct / max(1.0, self.wp)))
        lost_vwc = (hourly_etc_mm * ks) * mm_to_vwc

        # Update moisture before drainage
        new_moisture = self.current_moisture_pct + added_vwc - lost_vwc

        # 4. Gravitational Drainage:
        # Moisture above Field Capacity drains downward with a rapid decay rate
        drainage_mm = 0.0
        if new_moisture > self.fc:
            excess_vwc = new_moisture - self.fc
            # 25% of excess water drains per hour
            drain_vwc = excess_vwc * 0.25
            new_moisture -= drain_vwc
            drainage_mm = drain_vwc / mm_to_vwc

        # Cap within physical limits [air dry ~ 2%, saturation]
        new_moisture = max(2.0, min(self.sat, new_moisture))
        self.current_moisture_pct = new_moisture

        # 5. Diurnal Soil Temperature at 40cm depth:
        # Damped swing lagged by ~4 hours behind air temperature
        soil_temp = 18.0 + 3.5 * math.sin((hour - 10) / 24.0 * 2 * math.pi) + random.uniform(-0.1, 0.1)

        # 6. Battery charge/discharge cycle (solar panel recharge during 09:00 - 17:00)
        if 9 <= hour <= 17:
            self.battery_volts = min(3.65, self.battery_volts + 0.002)
        else:
            self.battery_volts = max(3.10, self.battery_volts - 0.0005)

        # Add minor capacitive sensor noise (+-0.15% VWC)
        noisy_moisture = round(self.current_moisture_pct + random.uniform(-0.15, 0.15), 1)

        soil_status = WaterBalanceEngine.analyze_soil_status(
            self.current_moisture_pct,
            self.field.crop_key,
            self.field.growth_stage,
            self.field.soil_key
        )

        return SimulationStepResult(
            timestamp=current_time,
            hour=hour,
            soil_moisture_pct=noisy_moisture,
            soil_temp_c=round(soil_temp, 1),
            battery_volts=round(self.battery_volts, 2),
            etc_mm=round(hourly_etc_mm, 2),
            rain_mm=round(rain_mm, 1),
            irrigation_mm=round(irrigation_mm, 1),
            drainage_mm=round(drainage_mm, 2),
            stress_index=soil_status.stress_index
        )

    def run_scenario(
        self,
        days: int = 14,
        scenario_type: str = 'drought'
    ) -> List[SimulationStepResult]:
        """Generate a multi-day timeline of hourly sensor readings."""
        results: List[SimulationStepResult] = []
        start_time = datetime.now(timezone.utc) - timedelta(days=days)

        for d in range(days):
            day_date = start_time + timedelta(days=d)
            
            # Weather pattern based on scenario
            if scenario_type == 'drought':
                et0 = 5.2 + (d * 0.1)
                air_max = 30.0 + min(5.0, d * 0.4)
                rain_day = 0.0
            elif scenario_type == 'heatwave_with_storm':
                et0 = 6.0 if d < 8 else 3.8
                air_max = 34.0 if d < 8 else 24.0
                rain_day = 18.5 if d == 8 else 0.0
            else: # normal summer
                et0 = 4.2 + (math.sin(d / 2.0) * 0.8)
                air_max = 26.0 + (math.sin(d / 2.0) * 3.0)
                rain_day = 8.0 if d in (4, 11) else 0.0

            for h in range(24):
                step_time = day_date.replace(hour=h, minute=0, second=0, microsecond=0)
                # Rain distributed in early morning or evening
                rain_hour = (rain_day / 3.0) if (rain_day > 0 and 14 <= h <= 16) else 0.0
                
                step_res = self.step_hour(
                    current_time=step_time,
                    et0_day_mm=et0,
                    air_temp_c=air_max,
                    rain_mm=rain_hour,
                    irrigation_mm=0.0
                )
                results.append(step_res)

        return results
