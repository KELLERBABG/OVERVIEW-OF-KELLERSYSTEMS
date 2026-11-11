from src.ingestion.lorawan_decoder import LoRaWANDecoder

def test_dragino_lse01_decoding():
    # Byte 0-1: 0x0E50 (3664 mV -> 3.66V)
    # Byte 2-3: 0x00E1 (225 -> 22.5 C)
    # Byte 4-5: 0x0834 (2100 -> 21.0% VWC)
    # Byte 6-7: 0x012C (300 uS/cm)
    raw_payload = bytes([0x0E, 0x50, 0x00, 0xE1, 0x08, 0x34, 0x01, 0x2C])
    decoded = LoRaWANDecoder.decode_dragino_lse01(raw_payload)
    
    assert decoded['battery_volts'] == 3.66
    assert decoded['soil_temp_c'] == 22.5
    assert decoded['soil_moisture_pct'] == 21.0
    assert decoded.get('soil_ec_us_cm') == 300

def test_ttn_webhook_decoding():
    ttn_payload = {
        'end_device_ids': {
            'device_id': 'probe-vineyard-01',
            'dev_eui': '2CF7F1C044300001'
        },
        'uplink_message': {
            'decoded_payload': {
                'soil_moisture': 23.4,
                'soil_temp': 21.2,
                'battery': 3.65
            },
            'rx_metadata': [
                {'rssi': -74, 'snr': 9.2}
            ]
        }
    }
    decoded = LoRaWANDecoder.decode_ttn_webhook(ttn_payload)
    assert decoded['deveui'] == '2CF7F1C044300001'
    assert decoded['soil_moisture_pct'] == 23.4
    assert decoded['soil_temp_c'] == 21.2
    assert decoded['battery_volts'] == 3.65
    assert decoded['rssi'] == -74
