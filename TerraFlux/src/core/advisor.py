"""Agronomic Decision Engine for FieldSense Irrigation."""
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, List, Optional
from .models import FieldModel
from .water_balance import WaterBalanceEngine, SoilWaterStatus

# Longest irrigation window used to check how much water the soil can take in one event.
MAX_IRRIGATION_WINDOW_HOURS = 6
@dataclass
class WeatherForecastSummary:
    et0_today_mm: float
    et0_tomorrow_mm: float
    rain_next_24h_mm: float   # precipitation of the current calendar day (Open-Meteo daily resolution)
    rain_next_48h_mm: float   # today + tomorrow
    rain_next_72h_mm: float = 0.0  # today + tomorrow + day after
    max_temp_tomorrow_c: float = 25.0
    solar_radiation_mj: float = 0.0

@dataclass
class AdvisoryDecision:
    should_irrigate: bool
    urgency: str # 'NONE', 'LOW', 'OPTIMAL', 'HIGH', 'CRITICAL'
    target_depth_mm: float      # NET depth required in the root zone (mm)
    gross_depth_mm: float       # GROSS depth the grower must run through the emitters (mm)
    duration_minutes: int
    optimal_start_time: str
    soil_status: SoilWaterStatus
    etc_tomorrow_mm: float
    rain_suppression: bool
    reason: str
    zone: Optional[str] = None      # irrigation zone the advice refers to
    valve_id: Optional[str] = None  # physical valve to open

def hours_since_reading(sensor: Any) -> float:
    """Hours since the probe last reported (0 when there is no usable timestamp)."""
    if sensor is None:
        return 0.0
    last_seen = getattr(sensor, 'last_seen', None)
    if not last_seen:
        return 0.0
    if last_seen.tzinfo is None:
        last_seen = last_seen.replace(tzinfo=timezone.utc)
    delta = datetime.now(timezone.utc) - last_seen
    return max(0.0, delta.total_seconds() / 3600.0)


class AgronomicAdvisor:
    """Translates soil physics & weather forecasts into precise watering advice."""

    @classmethod
    def select_critical_zone(cls, field: FieldModel, sensors: List[Any]):
        """Pick the zone that needs water first.

        Returns ``(sensor, zone, valve_id, moisture_pct, hours_since_reading)`` for the
        most stressed probe of a block, or ``None`` when the block has no probes.
        A probe without a zone represents the whole block.
        """
        if not sensors:
            return None

        best = None
        best_margin = None
        for sensor in sensors:
            status = WaterBalanceEngine.analyze_soil_status(
                current_moisture_pct=sensor.last_moisture_pct,
                crop_key=field.crop_key,
                growth_stage=field.growth_stage,
                soil_key=field.soil_key
            )
            # Margin to the stress threshold: negative means already below it.
            margin = sensor.last_moisture_pct - status.threshold_moisture_pct
            if best_margin is None or margin < best_margin:
                best_margin = margin
                best = sensor

        sensor = best
        return (
            sensor,
            sensor.zone,
            getattr(sensor, 'valve_id', None),
            sensor.last_moisture_pct,
            hours_since_reading(sensor)
        )

    @classmethod
    def evaluate_field(
        cls,
        field: FieldModel,
        current_moisture_pct: float,
        weather: WeatherForecastSummary,
        hours_since_reading: float = 0.0,
        zone: Optional[str] = None,
        valve_id: Optional[str] = None
    ) -> AdvisoryDecision:
        """Generate a crisp, scientific irrigation recommendation for a field.

        `hours_since_reading` allows the caller to compensate a stale probe reading
        with the crop demand and the rainfall since the uplink.
        """
        crop_params = WaterBalanceEngine.get_crop_params(field.crop_key)
        soil_params = WaterBalanceEngine.get_soil_params(field.soil_key)

        # Regulated Deficit Irrigation (RDI): the crop configuration may allow a
        # controlled deficit in the late stages. The depletion fraction is raised so
        # the crop is not watered back to field capacity too early.
        stress_strategy = str(crop_params.get('stress_strategy', ''))
        rdi_active = ('deficit' in stress_strategy.lower()) and field.growth_stage in ('late_season', 'development')
        base_p = float(crop_params.get('depletion_fraction_p', 0.50))
        p_override = min(0.80, base_p + 0.15) if rdi_active else None

        root_depth_m = float(crop_params.get('root_depth_m', 1.0))
        etc_today = WaterBalanceEngine.calculate_crop_etc(
            et0_mm=weather.et0_today_mm,
            crop_key=field.crop_key,
            growth_stage=field.growth_stage
        )
        # Correct the reading for what the weather did since the probe reported.
        adjusted_moisture_pct = WaterBalanceEngine.adjust_moisture_for_weather(
            current_moisture_pct=current_moisture_pct,
            hours_since_reading=hours_since_reading,
            etc_day_mm=etc_today,
            rain_mm=weather.rain_next_24h_mm,
            root_depth_m=root_depth_m
        )

        soil_status = WaterBalanceEngine.analyze_soil_status(
            current_moisture_pct=adjusted_moisture_pct,
            crop_key=field.crop_key,
            growth_stage=field.growth_stage,
            soil_key=field.soil_key,
            depletion_fraction_override=p_override
        )

        etc_tomorrow = WaterBalanceEngine.calculate_crop_etc(
            et0_mm=weather.et0_tomorrow_mm,
            crop_key=field.crop_key,
            growth_stage=field.growth_stage
        )

        # Expected moisture drop without irrigation tomorrow:
        # delta_theta = (ETc / (1000 * Zr)) * 100
        moisture_drop_pct = (etc_tomorrow / (1000.0 * max(0.2, soil_status.root_depth_m))) * 100.0
        predicted_moisture_tomorrow = adjusted_moisture_pct - moisture_drop_pct

        # Rain suppression logic:
        # 1) a substantial rain event in the next 48 h suppresses irrigation, and
        # 2) rain that covers most of the current deficit also suppresses it.
        rain_suppressed = False
        rain_cover_pct = 0.0
        if weather.rain_next_48h_mm >= 8.0:
            rain_suppressed = True
        if soil_status.depletion_mm > 0:
            rain_cover_pct = (weather.rain_next_72h_mm / soil_status.depletion_mm) * 100.0
            if rain_cover_pct >= 80.0 and adjusted_moisture_pct > soil_status.wilting_point_pct + 2.0:
                rain_suppressed = True

        # Decision Matrix
        should_irrigate = False
        urgency = 'NONE'
        target_depth_mm = 0.0
        reason = ''

        if rain_suppressed and adjusted_moisture_pct > soil_status.wilting_point_pct + 2.0:
            should_irrigate = False
            urgency = 'LOW'
            reason = (f'Suppressed: {weather.rain_next_48h_mm:.1f}mm rain forecast in next 48h '
                      f'({weather.rain_next_72h_mm:.1f}mm in 72h, {rain_cover_pct:.0f}% of the deficit). '
                      f'Soil reservoir at {adjusted_moisture_pct:.1f}%.')
        elif adjusted_moisture_pct <= soil_status.threshold_moisture_pct or predicted_moisture_tomorrow < soil_status.threshold_moisture_pct:
            should_irrigate = True
            
            # Refill target (NET depth in the root zone): replenish the depleted RAW to
            # bring the root zone back towards Field Capacity (or 90% FC for RDI).
            target_depth_mm = min(soil_status.raw_mm * 0.9, max(6.0, soil_status.depletion_mm * 0.8))
            # Round to clean integer/half-integer mm
            target_depth_mm = round(target_depth_mm * 2) / 2

            # The soil can only take what infiltrates during the irrigation window, and
            # the drip can only deliver so much per event. A bigger refill has to be
            # split over several cycles - a 30 hour valve run is not actionable.
            infiltration_rate_mm_h = float(soil_params.get('infiltration_rate_mm_h', 15.0))
            net_rate_mm_h = field.application_rate_mm_h
            max_by_window = net_rate_mm_h * MAX_IRRIGATION_WINDOW_HOURS
            max_by_infiltration = infiltration_rate_mm_h * field.irrigation_efficiency * MAX_IRRIGATION_WINDOW_HOURS
            max_event_depth_mm = max(1.0, min(max_by_window, max_by_infiltration))
            split_note = ''
            if target_depth_mm > max_event_depth_mm:
                cycles = int(target_depth_mm // max_event_depth_mm) + 1
                target_depth_mm = max_event_depth_mm
                split_note = (f' Split over {cycles} cycles: the drip delivers '
                              f'{field.gross_application_rate_mm_h:.1f} mm/h and the soil takes '
                              f'about {infiltration_rate_mm_h:.0f} mm/h.')

            if adjusted_moisture_pct <= (soil_status.wilting_point_pct + 1.0):
                urgency = 'CRITICAL'
                reason = f'Soil moisture ({adjusted_moisture_pct:.1f}%) near wilting point ({soil_status.wilting_point_pct:.1f}%). Immediate irrigation required.'
            elif adjusted_moisture_pct <= soil_status.threshold_moisture_pct:
                urgency = 'HIGH'
                reason = f'Soil moisture ({adjusted_moisture_pct:.1f}%) below RAW threshold ({soil_status.threshold_moisture_pct:.1f}%).'
            else:
                urgency = 'OPTIMAL'
                reason = f'Moisture ({adjusted_moisture_pct:.1f}%) will cross threshold tomorrow due to {etc_tomorrow:.1f}mm crop demand.'

            if rdi_active:
                reason += f' RDI active ({stress_strategy}).'
            if split_note:
                reason += split_note

            # Warn when the drip applies faster than the soil can absorb it.
            if field.gross_application_rate_mm_h > infiltration_rate_mm_h:
                reason += (f' Note: application rate {field.gross_application_rate_mm_h:.1f} mm/h exceeds the '
                           f'infiltration rate ({infiltration_rate_mm_h:.0f} mm/h) - expect surface runoff.')
        else:
            should_irrigate = False
            urgency = 'NONE'
            reason = f'Soil moisture optimal ({adjusted_moisture_pct:.1f}% > {soil_status.threshold_moisture_pct:.1f}% threshold). No watering needed.'
            if rdi_active:
                reason += f' RDI active ({stress_strategy}).'

        if abs(adjusted_moisture_pct - current_moisture_pct) >= 0.05:
            reason += (f' Reading corrected for {hours_since_reading:.0f} h of crop demand '
                       f'({current_moisture_pct:.1f}% -> {adjusted_moisture_pct:.1f}%).')

        # The grower sets the valve on a GROSS volume; only the share that infiltrates
        # reaches the root zone. Both numbers are reported so the advice is unambiguous.
        gross_depth_mm = WaterBalanceEngine.calculate_gross_depth_mm(
            net_depth_mm=target_depth_mm,
            irrigation_efficiency=field.irrigation_efficiency
        )
        gross_depth_mm = round(gross_depth_mm * 2) / 2

        # Compute duration: net depth delivered at the net application rate
        app_rate = field.application_rate_mm_h
        duration_minutes = WaterBalanceEngine.calculate_irrigation_duration_min(
            target_depth_mm=target_depth_mm,
            application_rate_mm_h=app_rate
        ) if should_irrigate else 0

        # Optimal start time: Early morning (05:30 - 06:30) to reduce evaporative loss and wind drift
        optimal_start_time = '06:00' if weather.max_temp_tomorrow_c >= 25.0 else '07:00'

        return AdvisoryDecision(
            should_irrigate=should_irrigate,
            urgency=urgency,
            target_depth_mm=target_depth_mm,
            gross_depth_mm=gross_depth_mm,
            duration_minutes=duration_minutes,
            optimal_start_time=optimal_start_time,
            soil_status=soil_status,
            etc_tomorrow_mm=round(etc_tomorrow, 1),
            rain_suppression=rain_suppressed,
            reason=reason,
            zone=zone,
            valve_id=valve_id
        )
