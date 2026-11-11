"""Tests for multi-depth probe support: weighted root zone, EC and deep percolation."""
import base64
import struct

import pytest
from sqlmodel import Session, select

from src.api.database import engine
from src.core.models import SensorModel, TelemetryReadingModel
from src.core.water_balance import WaterBalanceEngine
from src.ingestion.lorawan_decoder import LoRaWANDecoder


def _multi_depth_payload(battery_mv=3650, temp_dc=215, ec=850, m15=1800, m30=1900, m60=2100) -> bytes:
    """12 byte TerraFlux multi-depth frame."""
    return struct.pack('>HhHHHH', battery_mv, temp_dc, ec, m15, m30, m60)


def test_root_zone_moisture_weights_the_depths():
    value, source = WaterBalanceEngine.compute_root_zone_moisture(18.0, 20.0, 24.0)
    # 0.25*18 + 0.50*20 + 0.25*24 = 20.5
    assert value == 20.5
    assert source == 'multi_depth'


def test_root_zone_moisture_renormalises_partial_profiles():
    # Only 30 cm and 60 cm reported -> weights 0.5/0.25 renormalised to 2/3 and 1/3
    value, source = WaterBalanceEngine.compute_root_zone_moisture(None, 20.0, 26.0)
    assert source == 'multi_depth'
    assert value == pytest.approx((20.0 * 0.5 + 26.0 * 0.25) / 0.75, abs=0.01)


def test_root_zone_moisture_falls_back_to_single_depth():
    value, source = WaterBalanceEngine.compute_root_zone_moisture(None, None, None, fallback_pct=19.4)
    assert value == 19.4
    assert source == 'single_depth'


def test_deep_percolation_detection():
    assert WaterBalanceEngine.detect_deep_percolation(20.0, 23.0) is True
    assert WaterBalanceEngine.detect_deep_percolation(20.0, 21.5) is False
    assert WaterBalanceEngine.detect_deep_percolation(None, 23.0) is False
    assert WaterBalanceEngine.detect_deep_percolation(20.0, None) is False


def test_decoder_reads_the_multidepth_frame():
    decoded = LoRaWANDecoder.decode_terraflux_multidepth(_multi_depth_payload())
    assert decoded['battery_volts'] == 3.65
    assert decoded['soil_temp_c'] == 21.5
    assert decoded['soil_ec_us_cm'] == 850
    assert decoded['moisture_15cm_pct'] == 18.0
    assert decoded['moisture_30cm_pct'] == 19.0
    assert decoded['moisture_60cm_pct'] == 21.0


def test_decoder_rejects_a_short_frame():
    with pytest.raises(ValueError):
        LoRaWANDecoder.decode_terraflux_multidepth(b'\x00\x01\x02')


def test_ttn_webhook_decodes_multidepth_payload():
    body = {
        'end_device_ids': {'dev_eui': '2CF7F1C0443000AA'},
        'uplink_message': {
            'frm_payload': base64.b64encode(_multi_depth_payload()).decode()
        }
    }
    decoded = LoRaWANDecoder.decode_ttn_webhook(body)
    assert decoded['moisture_15cm_pct'] == 18.0
    assert decoded['moisture_60cm_pct'] == 21.0
    assert decoded['soil_ec_us_cm'] == 850


def test_ttn_webhook_reads_decoded_multidepth_fields():
    body = {
        'end_device_ids': {'dev_eui': '2CF7F1C0443000AB'},
        'uplink_message': {
            'decoded_payload': {
                'soil_moisture_15cm': 17.5,
                'soil_moisture_30cm': 19.5,
                'soil_moisture_60cm': 22.0,
                'soil_ec': 640,
                'soil_temp': 20.0,
                'battery': 3.7
            }
        }
    }
    decoded = LoRaWANDecoder.decode_ttn_webhook(body)
    assert decoded['moisture_15cm_pct'] == 17.5
    assert decoded['moisture_60cm_pct'] == 22.0
    assert decoded['soil_ec_us_cm'] == 640


def test_ingest_persists_the_profile_and_effective_moisture(client, monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'TTN_WEBHOOK_SECRET', None)

    payload = {
        'end_device_ids': {'dev_eui': 'MULTIDEPTH001'},
        'uplink_message': {
            'decoded_payload': {
                'soil_moisture_15cm': 18.0,
                'soil_moisture_30cm': 20.0,
                'soil_moisture_60cm': 24.0,
                'soil_ec': 700,
                'soil_temp': 21.0,
                'battery': 3.66
            }
        }
    }
    resp = client.post('/api/v1/telemetry/ttn', json=payload)
    assert resp.status_code == 200, resp.text
    data = resp.json()
    # effective root zone = 0.25*18 + 0.5*20 + 0.25*24 = 20.5
    assert data['moisture'] == 20.5
    assert data['moisture_source'] == 'multi_depth'

    with Session(engine) as session:
        sensor = session.exec(select(SensorModel).where(SensorModel.deveui == 'MULTIDEPTH001')).first()
        assert sensor.last_moisture_pct == 20.5
        assert sensor.moisture_15cm_pct == 18.0
        assert sensor.moisture_60cm_pct == 24.0
        assert sensor.soil_ec_us_cm == 700

        reading = session.exec(
            select(TelemetryReadingModel).where(TelemetryReadingModel.sensor_id == sensor.id)
        ).first()
        assert reading.soil_moisture_pct == 20.5
        assert reading.soil_ec_us_cm == 700
        assert reading.deep_percolation is False


def test_ingest_flags_deep_percolation(client, monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'TTN_WEBHOOK_SECRET', None)

    base = {
        'end_device_ids': {'dev_eui': 'PERCOLATION001'},
        'uplink_message': {'decoded_payload': {
            'soil_moisture_15cm': 22.0, 'soil_moisture_30cm': 24.0,
            'soil_moisture_60cm': 20.0, 'soil_temp': 20.0, 'battery': 3.6
        }}
    }
    assert client.post('/api/v1/telemetry/ttn', json=base).status_code == 200

    # Second uplink: the 60 cm layer gained 3 % -> water left the root zone
    risen = {
        'end_device_ids': {'dev_eui': 'PERCOLATION001'},
        'uplink_message': {'decoded_payload': {
            'soil_moisture_15cm': 24.0, 'soil_moisture_30cm': 26.0,
            'soil_moisture_60cm': 23.5, 'soil_temp': 19.0, 'battery': 3.6
        }}
    }
    resp = client.post('/api/v1/telemetry/ttn', json=risen)
    assert resp.json()['deep_percolation'] is True

    with Session(engine) as session:
        sensor = session.exec(select(SensorModel).where(SensorModel.deveui == 'PERCOLATION001')).first()
        assert sensor.deep_percolation is True


def test_single_depth_uplink_keeps_working(client, monkeypatch):
    from src.core.config import settings
    monkeypatch.setattr(settings, 'TTN_WEBHOOK_SECRET', None)

    payload = {
        'end_device_ids': {'dev_eui': 'SINGLEDEPTH001'},
        'uplink_message': {'decoded_payload': {'soil_moisture': 19.5, 'soil_temp': 20.0, 'battery': 3.6}}
    }
    resp = client.post('/api/v1/telemetry/ttn', json=payload)
    assert resp.status_code == 200
    assert resp.json()['moisture'] == 19.5
    assert resp.json()['moisture_source'] == 'single_depth'
    assert resp.json()['deep_percolation'] is False
