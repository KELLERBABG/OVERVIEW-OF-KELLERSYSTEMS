from src.ingestion.weather import WeatherService
from src.core.advisor import WeatherForecastSummary

def test_weather_cache_functionality():
    lat, lon = 48.0438, 7.6481
    # First call - fetches or fallbacks and stores in cache
    summary1 = WeatherService.get_forecast(lat, lon)
    assert isinstance(summary1, WeatherForecastSummary)
    
    # Check cache key
    key = WeatherService._cache_key(lat, lon)
    assert key in WeatherService._cache

    # Second call should immediately hit cache
    summary2 = WeatherService.get_forecast(lat, lon)
    assert summary1.et0_tomorrow_mm == summary2.et0_tomorrow_mm
    assert summary1.rain_next_48h_mm == summary2.rain_next_48h_mm
