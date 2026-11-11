"""LoRaWAN Telemetry Payload Decoders (Dragino LSE01, Cayenne LPP, TTN v3)."""
import base64
from typing import Dict, Any

class LoRaWANDecoder:
    """Decodes uplinks from commercial and DIY agricultural soil sensors."""

    @classmethod
    def decode_dragino_lse01(cls, payload_bytes: bytes) -> Dict[str, Any]:
        """
        Decodes Dragino LSE01 Soil Moisture & EC Sensor.
        Payload structure (11 bytes):
        - Bytes 0-1: Battery voltage (mV) -> (val & 0x3FFF) / 1000.0
        - Bytes 2-3: Soil Temp (signed short) / 10.0 or / 100.0
        - Bytes 4-5: Soil Moisture VWC (0.00 - 100.00%) / 100.0 or / 10.0
        - Bytes 6-7: Soil Conductivity (EC) uS/cm
        """
        if len(payload_bytes) < 6:
            raise ValueError(f'Payload too short for Dragino LSE01: {len(payload_bytes)} bytes')

        bat_raw = int.from_bytes(payload_bytes[0:2], byteorder='big')
        battery_volts = (bat_raw & 0x3FFF) / 1000.0

        temp_raw = int.from_bytes(payload_bytes[2:4], byteorder='big', signed=True)
        soil_temp = temp_raw / 10.0 if abs(temp_raw) < 1000 else temp_raw / 100.0

        moisture_raw = int.from_bytes(payload_bytes[4:6], byteorder='big')
        soil_moisture = moisture_raw / 100.0 if moisture_raw > 100 else float(moisture_raw)

        soil_ec = None
        if len(payload_bytes) >= 8:
            soil_ec = int.from_bytes(payload_bytes[6:8], byteorder='big')

        result = {
            'battery_volts': round(battery_volts, 2),
            'soil_temp_c': round(soil_temp, 1),
            'soil_moisture_pct': round(soil_moisture, 1)
        }
        if soil_ec is not None:
            result['soil_ec_us_cm'] = soil_ec
        return result

    @classmethod
    def decode_cayenne_lpp(cls, payload_bytes: bytes) -> Dict[str, Any]:
        """
        Decodes Cayenne Low Power Payload (LPP).
        Channel 1: Analog Input (Soil Moisture %)
        Channel 2: Temperature (Soil Temp C)
        Channel 3: Analog Input (Battery V)
        """
        result = {'soil_moisture_pct': 20.0, 'soil_temp_c': 20.0, 'battery_volts': 3.6}
        i = 0
        while i < len(payload_bytes):
            channel = payload_bytes[i]
            data_type = payload_bytes[i+1]
            if data_type == 0x02: # Analog Input (Moisture or Voltage)
                val = int.from_bytes(payload_bytes[i+2:i+4], byteorder='big', signed=True) / 100.0
                if channel == 1:
                    result['soil_moisture_pct'] = round(val, 1)
                elif channel == 3:
                    result['battery_volts'] = round(val, 2)
                i += 4
            elif data_type == 0x67: # Temperature Sensor (0.1 C)
                temp = int.from_bytes(payload_bytes[i+2:i+4], byteorder='big', signed=True) / 10.0
                result['soil_temp_c'] = round(temp, 1)
                i += 4
            elif data_type == 0x74: # Voltage (0.01 V)
                v = int.from_bytes(payload_bytes[i+2:i+4], byteorder='big') / 100.0
                result['battery_volts'] = round(v, 2)
                i += 4
            else:
                i += 3 # skip unknown
        return result

    @classmethod
    def decode_terraflux_multidepth(cls, payload_bytes: bytes) -> Dict[str, Any]:
        """Decodes the TerraFlux multi-depth profile (DIY ESP32 / LoRaWAN node).

        Payload structure (12 bytes, big endian, unsigned):
        - Bytes 0-1: Battery voltage in mV (0.1 V steps -> mV/1000)
        - Bytes 2-3: Soil temperature in 0.1 °C (signed)
        - Bytes 4-5: Soil EC in µS/cm
        - Bytes 6-7: VWC at 15 cm (0.01 %)
        - Bytes 8-9: VWC at 30 cm (0.01 %)
        - Bytes 10-11: VWC at 60 cm (0.01 %)
        """
        if len(payload_bytes) < 12:
            raise ValueError(f'Payload too short for the multi-depth profile: {len(payload_bytes)} bytes')

        battery_volts = int.from_bytes(payload_bytes[0:2], byteorder='big') / 1000.0
        soil_temp = int.from_bytes(payload_bytes[2:4], byteorder='big', signed=True) / 10.0
        soil_ec = int.from_bytes(payload_bytes[4:6], byteorder='big')
        moisture_15 = int.from_bytes(payload_bytes[6:8], byteorder='big') / 100.0
        moisture_30 = int.from_bytes(payload_bytes[8:10], byteorder='big') / 100.0
        moisture_60 = int.from_bytes(payload_bytes[10:12], byteorder='big') / 100.0

        return {
            'battery_volts': round(battery_volts, 2),
            'soil_temp_c': round(soil_temp, 1),
            'soil_ec_us_cm': soil_ec,
            'moisture_15cm_pct': round(moisture_15, 1),
            'moisture_30cm_pct': round(moisture_30, 1),
            'moisture_60cm_pct': round(moisture_60, 1)
        }

    @classmethod
    def decode_ttn_webhook(cls, body: Dict[str, Any]) -> Dict[str, Any]:
        """
        Extracts telemetry fields from a standard TTN v3 (The Things Stack) uplink webhook.
        """
        end_device_ids = body.get('end_device_ids', {})
        deveui = end_device_ids.get('dev_eui', 'UNKNOWN_EUI')
        device_id = end_device_ids.get('device_id', deveui)

        uplink = body.get('uplink_message', {})
        decoded_payload = uplink.get('decoded_payload') or {}
        frm_payload_b64 = uplink.get('frm_payload')

        moisture = decoded_payload.get('soil_moisture') or decoded_payload.get('moisture') or decoded_payload.get('water_content')
        temp = decoded_payload.get('soil_temp') or decoded_payload.get('temperature')
        battery = decoded_payload.get('battery') or decoded_payload.get('battery_voltage') or decoded_payload.get('vbat')

        # Multi-depth profile (optional)
        moisture_15 = decoded_payload.get('soil_moisture_15cm') or decoded_payload.get('moisture_15cm')
        moisture_30 = decoded_payload.get('soil_moisture_30cm') or decoded_payload.get('moisture_30cm')
        moisture_60 = decoded_payload.get('soil_moisture_60cm') or decoded_payload.get('moisture_60cm')
        ec = decoded_payload.get('soil_ec') or decoded_payload.get('soil_ec_us_cm') or decoded_payload.get('ec')

        if moisture is None and frm_payload_b64:
            try:
                raw_bytes = base64.b64decode(frm_payload_b64)
                if len(raw_bytes) >= 12:
                    parsed = cls.decode_terraflux_multidepth(raw_bytes)
                else:
                    parsed = cls.decode_dragino_lse01(raw_bytes)
                moisture = parsed.get('soil_moisture_pct')
                temp = parsed.get('soil_temp_c')
                battery = parsed.get('battery_volts')
                moisture_15 = parsed.get('moisture_15cm_pct', moisture_15)
                moisture_30 = parsed.get('moisture_30cm_pct', moisture_30)
                moisture_60 = parsed.get('moisture_60cm_pct', moisture_60)
                ec = parsed.get('soil_ec_us_cm', ec)
            except Exception:
                pass

        # Metadata
        rx_metadata = uplink.get('rx_metadata', [{}])[0]
        rssi = rx_metadata.get('rssi', -75)
        snr = rx_metadata.get('snr', 9.5)

        def _as_float(value, default):
            try:
                return float(value) if value is not None else default
            except (TypeError, ValueError):
                return default

        result = {
            'deveui': deveui,
            'device_id': device_id,
            'soil_moisture_pct': _as_float(moisture, 22.0),
            'soil_temp_c': _as_float(temp, 20.5),
            'battery_volts': _as_float(battery, 3.65),
            'rssi': rssi,
            'snr': snr
        }
        if moisture_15 is not None:
            result['moisture_15cm_pct'] = _as_float(moisture_15, 22.0)
        if moisture_30 is not None:
            result['moisture_30cm_pct'] = _as_float(moisture_30, 22.0)
        if moisture_60 is not None:
            result['moisture_60cm_pct'] = _as_float(moisture_60, 22.0)
        if ec is not None:
            result['soil_ec_us_cm'] = _as_float(ec, 0.0)
        return result
