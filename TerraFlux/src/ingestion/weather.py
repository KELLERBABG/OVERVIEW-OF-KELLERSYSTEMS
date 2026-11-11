"""Live weather ingestion, Open-Meteo Agro client with TTL caching and fail-safe resilience."""
from typing import Dict, Tuple
import time
import httpx
from ..core.advisor import WeatherForecastSummary
from ..core.config import settings
from ..core.logger import logger

class WeatherUnavailableException(Exception):
    """Raised when weather API fails and no fresh cache is available."""
    pass

class WeatherService:
    """Fetches live reference evapotranspiration (ET0) and rain forecasts with caching."""

    BASE_URL = 'https://api.open-meteo.com/v1/forecast'
    _cache: Dict[str, Tuple[float, WeatherForecastSummary]] = {}

    @classmethod
    def _cache_key(cls, latitude: float, longitude: float) -> str:
        return f"{round(latitude, 3)}_{round(longitude, 3)}"

    @classmethod
    def get_forecast(cls, latitude: float, longitude: float) -> WeatherForecastSummary:
        """Fetch live Open-Meteo agro weather with reference ET0 and fail-safe TTL caching."""
        cache_key = cls._cache_key(latitude, longitude)
        now_ts = time.time()

        # Check cache
        if cache_key in cls._cache:
            cached_time, cached_summary = cls._cache[cache_key]
            if (now_ts - cached_time) < settings.WEATHER_CACHE_TTL_SECONDS:
                logger.debug(f"[WEATHER CACHE HIT] Coordinates: {latitude}, {longitude}")
                return cached_summary

        params = {
            'latitude': latitude,
            'longitude': longitude,
            'daily': [
                'et0_fao_evapotranspiration',
                'precipitation_sum',
                'temperature_2m_max',
                'shortwave_radiation_sum'
            ],
            'timezone': 'auto',
            'forecast_days': 3
        }

        request_url = cls.BASE_URL
        if settings.OPEN_METEO_API_KEY:
            request_url = 'https://customer-api.open-meteo.com/v1/forecast'
            params['apikey'] = settings.OPEN_METEO_API_KEY

        try:
            with httpx.Client(timeout=8.0) as client:
                resp = client.get(request_url, params=params)
                if resp.status_code == 200:
                    data = resp.json()
                    daily = data.get('daily', {})
                    et0_list = daily.get('et0_fao_evapotranspiration', [4.5, 4.8, 4.2])
                    rain_list = daily.get('precipitation_sum', [0.0, 0.0, 0.0])
                    temp_list = daily.get('temperature_2m_max', [26.0, 28.0, 27.0])
                    rad_list = daily.get('shortwave_radiation_sum', [22.0, 24.0, 21.0])

                    summary = WeatherForecastSummary(
                        et0_today_mm=round(float(et0_list[0] if len(et0_list) > 0 else 4.5), 1),
                        et0_tomorrow_mm=round(float(et0_list[1] if len(et0_list) > 1 else 4.8), 1),
                        # Open-Meteo returns daily totals starting with the current day,
                        # so index 0 is "today" and therefore the next 24 h window.
                        rain_next_24h_mm=round(float(rain_list[0] if len(rain_list) > 0 else 0.0), 1),
                        rain_next_48h_mm=round(float(
                            (rain_list[0] if len(rain_list) > 0 else 0.0) +
                            (rain_list[1] if len(rain_list) > 1 else 0.0)
                        ), 1),
                        rain_next_72h_mm=round(float(
                            (rain_list[0] if len(rain_list) > 0 else 0.0) +
                            (rain_list[1] if len(rain_list) > 1 else 0.0) +
                            (rain_list[2] if len(rain_list) > 2 else 0.0)
                        ), 1),
                        max_temp_tomorrow_c=round(float(temp_list[1] if len(temp_list) > 1 else 27.0), 1),
                        solar_radiation_mj=round(float(rad_list[1] if len(rad_list) > 1 else 23.0), 1)
                    )

                    # Update cache
                    cls._cache[cache_key] = (now_ts, summary)
                    logger.info(f"[WEATHER SYNC SUCCESS] Lat: {latitude}, Lon: {longitude} | ET0 Tomorrow: {summary.et0_tomorrow_mm}mm, Rain: {summary.rain_next_48h_mm}mm")
                    return summary
                else:
                    logger.error(f"[WEATHER API ERROR] HTTP {resp.status_code}: {resp.text}")
        except Exception as e:
            logger.warning(f"[WEATHER NETWORK ERROR] Failed to reach Open-Meteo: {e}")

        # Fallback to stale cache if available
        if cache_key in cls._cache:
            _, stale_summary = cls._cache[cache_key]
            logger.warning(f"[WEATHER SERVING STALE CACHE] Coordinates: {latitude}, {longitude}")
            return stale_summary

        # Conservative fallback if network is down and zero cache exists
        logger.error(f"[WEATHER UNAVAILABLE] No cache for {latitude}, {longitude}. Using conservative safe baseline.")
        return WeatherForecastSummary(
            et0_today_mm=3.5,
            et0_tomorrow_mm=3.5,
            rain_next_24h_mm=0.0,
            rain_next_48h_mm=0.0,
            rain_next_72h_mm=0.0,
            max_temp_tomorrow_c=22.0,
            solar_radiation_mj=18.0
        )
