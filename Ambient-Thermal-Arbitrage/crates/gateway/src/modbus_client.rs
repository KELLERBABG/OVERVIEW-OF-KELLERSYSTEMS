//! Minimal native Modbus TCP client for the PLC interface.
//!
//! Implements the small subset of the Modbus Application Protocol needed by
//! the ATA gateway:
//!   - FC 0x03  Read Holding Registers   (internal & ambient temperature)
//!   - FC 0x05  Write Single Coil        (compressor command, alarm)
//!   - FC 0x06  Write Single Register    (VFD compressor speed, when configured)
//!
//! The PLC exposes the sensors as signed int16 registers scaled by 100
//! (`°C × 100`). The background polling loop mirrors the gateway control
//! state to the PLC coils/registers while logging the PLC sensor readings.
//!
//! The loop reconnects automatically with exponential backoff and never
//! panics — a lost PLC connection degrades gracefully to log-only mode.

use crate::config::ModbusConfig;
use std::sync::Arc;
use std::time::Duration;

// Modbus function codes used by this client.
const FC_READ_HOLDING_REGISTERS: u8 = 0x03;
const FC_WRITE_SINGLE_COIL: u8 = 0x05;
const FC_WRITE_SINGLE_REGISTER: u8 = 0x06;

// Connection / protocol constants.
const MBAP_HEADER_LEN: usize = 7;
const PROTOCOL_ID: u16 = 0;
pub const DEFAULT_UNIT_ID: u8 = 1;

/// Maximum reconnect backoff (seconds).
const MAX_CONNECT_BACKOFF_SEC: u64 = 30;

/// Errors produced by the Modbus client.
#[derive(Debug, thiserror::Error)]
pub enum ModbusError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("connection error: {0}")]
    Connect(String),
    #[error("invalid response from PLC (expected {expected} bytes, got {actual})")]
    InvalidResponse { expected: usize, actual: usize },
    #[error("PLC returned exception code {0}")]
    Exception(u8),
    #[error("transaction ID mismatch (sent {sent:#06x}, received {received:#06x})")]
    TransactionMismatch { sent: u16, received: u16 },
    #[error("protocol ID mismatch (got {0:#06x})")]
    ProtocolMismatch(u16),
    #[error("unit ID mismatch (sent {sent}, received {received})")]
    UnitMismatch { sent: u8, received: u8 },
    #[error("request timed out")]
    Timeout,
    #[error("write verification failed: PLC did not echo the coil value")]
    WriteVerifyFailed,
    #[error("CRC mismatch: calculated {calculated:#06x}, received {received:#06x}")]
    CrcMismatch { calculated: u16, received: u16 },
}

/// Standard Modbus RTU CRC-16 (polynomial 0xA001, initial 0xFFFF).
pub fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= b as u16;
        for _ in 0..8 {
            if (crc & 0x0001) != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// Modbus RTU Codec for constructing and parsing RS-485 serial frames.
pub struct ModbusRtuCodec;

impl ModbusRtuCodec {
    /// Encode a Modbus RTU ADU from a unit ID and PDU: `[Unit_ID, PDU..., CRC_lo, CRC_hi]`.
    pub fn encode_frame(unit_id: u8, pdu: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(1 + pdu.len() + 2);
        frame.push(unit_id);
        frame.extend_from_slice(pdu);
        let crc = calculate_crc16(&frame);
        frame.push((crc & 0xFF) as u8);
        frame.push(((crc >> 8) & 0xFF) as u8);
        frame
    }

    /// Decode and validate a Modbus RTU ADU: verifies CRC, checks unit ID,
    /// checks for exception codes, and returns the unpacked response PDU.
    pub fn decode_frame(unit_id: u8, raw: &[u8]) -> Result<Vec<u8>, ModbusError> {
        if raw.len() < 4 {
            return Err(ModbusError::InvalidResponse { expected: 4, actual: raw.len() });
        }
        let data_len = raw.len() - 2;
        let calculated_crc = calculate_crc16(&raw[..data_len]);
        let received_crc = (raw[data_len] as u16) | ((raw[data_len + 1] as u16) << 8);
        if calculated_crc != received_crc {
            return Err(ModbusError::CrcMismatch {
                calculated: calculated_crc,
                received: received_crc,
            });
        }
        if raw[0] != unit_id {
            return Err(ModbusError::UnitMismatch { sent: unit_id, received: raw[0] });
        }
        let pdu = &raw[1..data_len];
        if !pdu.is_empty() && (pdu[0] & 0x80) != 0 {
            let code = *pdu.get(1).unwrap_or(&0);
            return Err(ModbusError::Exception(code));
        }
        Ok(pdu.to_vec())
    }

    /// Build an RTU Read Holding Registers (FC 0x03) frame.
    pub fn build_read_holding_registers(unit_id: u8, address: u16, quantity: u16) -> Vec<u8> {
        let mut pdu = Vec::with_capacity(5);
        pdu.push(FC_READ_HOLDING_REGISTERS);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(&quantity.to_be_bytes());
        Self::encode_frame(unit_id, &pdu)
    }

    /// Build an RTU Write Single Coil (FC 0x05) frame.
    pub fn build_write_single_coil(unit_id: u8, address: u16, value: bool) -> Vec<u8> {
        let mut pdu = Vec::with_capacity(5);
        pdu.push(FC_WRITE_SINGLE_COIL);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(if value { &[0xFF, 0x00] } else { &[0x00, 0x00] });
        Self::encode_frame(unit_id, &pdu)
    }

    /// Build an RTU Write Single Register (FC 0x06) frame.
    pub fn build_write_single_register(unit_id: u8, address: u16, value: u16) -> Vec<u8> {
        let mut pdu = Vec::with_capacity(5);
        pdu.push(FC_WRITE_SINGLE_REGISTER);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(&value.to_be_bytes());
        Self::encode_frame(unit_id, &pdu)
    }
}

/// Per-request timeout: a silent PLC / half-open TCP socket must not stall the
/// loop indefinitely (default TCP retries can hang for 10+ minutes).
const REQUEST_TIMEOUT_SECS: u64 = 5;

/// Decode a raw register value (°C × 100, signed int16) into degC.
fn reg_to_c(value: i16) -> f64 {
    value as f64 / 100.0
}

/// Minimal Modbus TCP client (single connection, fixed unit id).
struct ModbusTcpClient {
    stream: tokio::net::TcpStream,
    tx_id: u16,
    unit_id: u8,
}

impl ModbusTcpClient {
    /// Connect to the PLC (bounded by a timeout).
    async fn connect(host: &str, port: u16, unit_id: u8) -> Result<Self, ModbusError> {
        let addr = format!("{}:{}", host, port);
        let stream = tokio::time::timeout(
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| ModbusError::Timeout)?
        .map_err(|e| ModbusError::Connect(format!("{} → {}", addr, e)))?;
        stream.set_nodelay(true).ok();

        log::info!("🔌 Modbus TCP connected to {}", addr);
        Ok(Self {
            stream,
            tx_id: 0,
            unit_id,
        })
    }

    fn next_tx_id(&mut self) -> u16 {
        self.tx_id = self.tx_id.wrapping_add(1);
        self.tx_id
    }

    /// Read a 16-bit holding register as a signed integer.
    async fn read_register(&mut self, address: u16) -> Result<i16, ModbusError> {
        let tx_id = self.next_tx_id();
        let mut pdu = Vec::with_capacity(5);
        pdu.push(FC_READ_HOLDING_REGISTERS);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(&1u16.to_be_bytes()); // quantity = 1

        let response = self.transact(tx_id, &pdu).await?;
        // Response PDU: FC(1) + byte_count(1) + data(2 bytes). Any other shape
        // (short/truncated frames, wrong FC) is invalid — index safety first:
        // the old `len < 3` guard still allowed response[3] to panic.
        if response.len() != 4 || response[0] != FC_READ_HOLDING_REGISTERS {
            return Err(ModbusError::InvalidResponse { expected: 4, actual: response.len() });
        }
        if response[1] as usize != 2 {
            return Err(ModbusError::InvalidResponse { expected: 2, actual: response[1] as usize });
        }
        Ok(i16::from_be_bytes([response[2], response[3]]))
    }

    /// Write a single coil (0x05). `value == true` → ON (0xFF00), else OFF (0x0000).
    async fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ModbusError> {
        let tx_id = self.next_tx_id();
        let mut pdu = Vec::with_capacity(6);
        pdu.push(FC_WRITE_SINGLE_COIL);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(if value { &[0xFF, 0x00] } else { &[0x00, 0x00] });

        let response = self.transact(tx_id, &pdu).await?;
        // FC05 echo PDU is exactly 5 bytes: FC + address(2) + value(2).
        // (The MBAP unit id is stripped by transact, so len==5, not 6.)
        if response.len() != 5 || response[0] != FC_WRITE_SINGLE_COIL {
            return Err(ModbusError::InvalidResponse { expected: 5, actual: response.len() });
        }
        if response[3..] != [if value { 0xFF } else { 0x00 }, 0x00] {
            return Err(ModbusError::WriteVerifyFailed);
        }
        Ok(())
    }

    /// Write a single holding register (0x06) — e.g. a VFD compressor speed
    /// command (uint16, 0..1000 = 0.0..100.0 %). The PLC must echo the write
    /// exactly (FC + address + value); any mismatch is a verified failure.
    async fn write_register(&mut self, address: u16, value: u16) -> Result<(), ModbusError> {
        let tx_id = self.next_tx_id();
        let mut pdu = Vec::with_capacity(6);
        pdu.push(FC_WRITE_SINGLE_REGISTER);
        pdu.extend_from_slice(&address.to_be_bytes());
        pdu.extend_from_slice(&value.to_be_bytes());

        let response = self.transact(tx_id, &pdu).await?;
        // FC06 echo PDU is exactly 5 bytes: FC + address(2) + value(2).
        if response.len() != 5 || response[0] != FC_WRITE_SINGLE_REGISTER {
            return Err(ModbusError::InvalidResponse { expected: 5, actual: response.len() });
        }
        if response[1..3] != address.to_be_bytes() || response[3..5] != value.to_be_bytes() {
            return Err(ModbusError::WriteVerifyFailed);
        }
        Ok(())
    }

    /// Send a PDU and read back the response PDU for the same transaction.
    /// The whole exchange is bounded by a timeout so a silent PLC cannot stall
    /// the polling loop.
    async fn transact(&mut self, tx_id: u16, pdu: &[u8]) -> Result<Vec<u8>, ModbusError> {
        tokio::time::timeout(
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
            self.transact_inner(tx_id, pdu),
        )
        .await
        .map_err(|_| ModbusError::Timeout)?
    }

    async fn transact_inner(&mut self, tx_id: u16, pdu: &[u8]) -> Result<Vec<u8>, ModbusError> {
        // MBAP header: tx_id(2) + protocol_id(2) + length(2) + unit_id(1)
        let length = (pdu.len() + 1) as u16; // pdu + unit id
        let mut frame = Vec::with_capacity(MBAP_HEADER_LEN + pdu.len());
        frame.extend_from_slice(&tx_id.to_be_bytes());
        frame.extend_from_slice(&PROTOCOL_ID.to_be_bytes());
        frame.extend_from_slice(&length.to_be_bytes());
        frame.push(self.unit_id);
        frame.extend_from_slice(pdu);

        use tokio::io::AsyncWriteExt;
        self.stream.write_all(&frame).await?;
        self.stream.flush().await?;

        use tokio::io::AsyncReadExt;
        let mut header = [0u8; MBAP_HEADER_LEN];
        self.stream.read_exact(&mut header).await?;

        let recv_tx_id = u16::from_be_bytes([header[0], header[1]]);
        let recv_proto = u16::from_be_bytes([header[2], header[3]]);
        let recv_len = u16::from_be_bytes([header[4], header[5]]) as usize;
        let recv_unit = header[6];

        if recv_tx_id != tx_id {
            return Err(ModbusError::TransactionMismatch { sent: tx_id, received: recv_tx_id });
        }
        if recv_proto != PROTOCOL_ID {
            return Err(ModbusError::ProtocolMismatch(recv_proto));
        }
        if recv_unit != self.unit_id {
            return Err(ModbusError::UnitMismatch { sent: self.unit_id, received: recv_unit });
        }
        // Modbus ADU max is 254 bytes (unit id + 253-byte PDU). A larger claimed
        // length means a garbage/spoofed frame — reject before allocating.
        if recv_len == 0 || recv_len > 254 {
            return Err(ModbusError::InvalidResponse { expected: 254, actual: recv_len });
        }

        let mut body = vec![0u8; recv_len - 1]; // minus unit id already read
        self.stream.read_exact(&mut body).await?;

        // Exception responses have the function code with the high bit set.
        if !body.is_empty() && (body[0] & 0x80) != 0 {
            let code = *body.get(1).unwrap_or(&0);
            return Err(ModbusError::Exception(code));
        }

        Ok(body)
    }
}

/// Callback receiving each successfully polled PLC reading:
/// (internal_temp_c, ambient_temp_c, grid_frequency_hz_opt).
pub type ModbusReadingCallback = Option<Arc<dyn Fn(f64, f64, Option<f64>) + Send + Sync>>;

/// Run the background Modbus polling loop.
///
/// `on_reading` receives each successfully read (internal, ambient,
/// frequency) triple — frequency is `None` when no `grid_frequency_addr`
/// register is configured. Used by the gateway to close the loop on real PLC
/// sensors when `sensor.simulate = false`.
pub async fn run_modbus_loop<F>(
    cfg: ModbusConfig,
    state_fn: Arc<F>,
    on_reading: ModbusReadingCallback,
) where
    F: Fn() -> (f64, f64, bool, Option<f64>, bool) + Send + Sync + 'static,
{
    if !cfg.enabled {
        return;
    }
    let host = cfg.host.clone();
    let port = cfg.port;
    let unit_id = cfg.unit_id;
    let poll_interval = Duration::from_secs_f64(cfg.poll_interval_sec.max(1.0));
    let regs = cfg.registers;
    let mut backoff_sec: u64 = 1;

    if cfg.protocol.eq_ignore_ascii_case("rtu") {
        log::info!(
            "🔌 Modbus configured for RTU serial mode (port: {}, baud: {}, unit: {})",
            cfg.serial_port.as_deref().unwrap_or("/dev/ttyUSB0"),
            cfg.baud_rate,
            unit_id
        );
    } else {
        log::info!("🔌 Modbus configured for TCP mode ({}:{}, unit: {})", host, port, unit_id);
    }

    let mut heartbeat_counter: u16 = 0;

    loop {
        let mut client = match ModbusTcpClient::connect(&host, port, unit_id).await {
            // Backoff is NOT reset here: a PLC that accepts TCP but never
            // responds would otherwise cause a tight reconnect loop.
            Ok(c) => c,
            Err(e) => {
                log::warn!("🔌 Modbus connect failed: {}", e);
                tokio::time::sleep(Duration::from_secs(backoff_sec)).await;
                backoff_sec = (backoff_sec * 2).min(MAX_CONNECT_BACKOFF_SEC);
                continue;
            }
        };
        // Per-connection write cache: ensure fresh sync on reconnect, COV during session
        let mut last_written_speed: Option<u16> = None;
        let mut last_written_coil: Option<bool> = None;
        let mut last_written_alarm: Option<bool> = None;

        'connected: loop {
            let internal = match client.read_register(regs.internal_temp_addr).await {
                Ok(v) => reg_to_c(v),
                Err(e) => { log::warn!("🔌 Modbus read internal temp failed: {}", e); break 'connected; }
            };
            let ambient = match client.read_register(regs.ambient_temp_addr).await {
                Ok(v) => reg_to_c(v),
                Err(e) => { log::warn!("🔌 Modbus read ambient temp failed: {}", e); break 'connected; }
            };
            // Optional real grid-frequency input: signed int16, Hz × 100
            // (5000 = 50.00 Hz). When configured, GridBalancing acts on the
            // meter's ACTUAL frequency instead of the simulated dataset.
            let frequency = match regs.grid_frequency_addr {
                Some(addr) => match client.read_register(addr).await {
                    Ok(v) => Some(v as f64 / 100.0),
                    Err(e) => {
                        log::warn!("🔌 Modbus read grid frequency failed: {}", e);
                        break 'connected;
                    }
                },
                None => None,
            };
            // A full poll cycle succeeded — reset the reconnect backoff.
            backoff_sec = 1;
            if let Some(cb) = &on_reading {
                cb(internal, ambient, frequency);
            }
            let (internal_gw, ambient_gw, compressor_on, compressor_speed, alarm) = state_fn();
            log::info!(
                "🔌 PLC: int {:.1}°C (gw {:.1}°C)  amb {:.1}°C (gw {:.1}°C)  comp {}  speed {}  alarm {}",
                internal, internal_gw, ambient, ambient_gw,
                if compressor_on { "ON" } else { "OFF" },
                compressor_speed.map(|s| format!("{:.0}%", s * 100.0)).unwrap_or_else(|| "coil".into()),
                if alarm { "YES" } else { "no" },
            );

            // ── Change-of-Value (COV) write-filter ──
            // Prevents hardware wearout on PLC EEPROMs/flash memory: write ONLY when
            // target speed or coil state changes, instead of writing every 2 seconds unconditionally.
            match regs.compressor_speed_addr {
                Some(addr) => {
                    let raw = match compressor_speed {
                        Some(s) => (s.clamp(0.0, 1.0) * 1000.0) as u16,
                        None => if compressor_on { 1000 } else { 0 },
                    };
                    if last_written_speed != Some(raw) {
                        if let Err(e) = client.write_register(addr, raw).await {
                            log::warn!("🔌 Modbus write compressor speed register failed: {}", e);
                            break 'connected;
                        }
                        last_written_speed = Some(raw);
                    }
                }
                None => {
                    if last_written_coil != Some(compressor_on) {
                        if let Err(e) = client.write_coil(regs.compressor_cmd_addr, compressor_on).await {
                            log::warn!("🔌 Modbus write compressor coil failed: {}", e);
                            break 'connected;
                        }
                        last_written_coil = Some(compressor_on);
                    }
                }
            }

            if last_written_alarm != Some(alarm) {
                if let Err(e) = client.write_coil(regs.alarm_addr, alarm).await {
                    log::warn!("🔌 Modbus write alarm coil failed: {}", e);
                    break 'connected;
                }
                last_written_alarm = Some(alarm);
            }

            // ── Heartbeat / Watchdog register ──
            // Increments counter every successful poll so PLC local watchdog can detect gateway freeze.
            if let Some(hb_addr) = regs.heartbeat_addr {
                heartbeat_counter = heartbeat_counter.wrapping_add(1);
                if let Err(e) = client.write_register(hb_addr, heartbeat_counter).await {
                    log::warn!("🔌 Modbus write heartbeat register failed: {}", e);
                    break 'connected;
                }
            }

            crate::metrics::inc_http("/modbus/poll", 200);
            tokio::time::sleep(poll_interval).await;
        }
        log::warn!("🔌 Modbus connection lost — reconnecting in {}s", backoff_sec);
        tokio::time::sleep(Duration::from_secs(backoff_sec)).await;
        backoff_sec = (backoff_sec * 2).min(MAX_CONNECT_BACKOFF_SEC);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg_to_c_positive() {
        assert_eq!(reg_to_c(100), 1.0);
        assert_eq!(reg_to_c(-2000), -20.0);
        assert_eq!(reg_to_c(0), 0.0);
    }

    #[test]
    fn test_reg_to_c_negative_scaled() {
        // -24.5°C → -2450
        assert_eq!(reg_to_c(-2450), -24.5);
    }

    #[tokio::test]
    async fn test_connect_failure_does_not_panic() {
        // Connecting to a closed port should return an error, not panic.
        let result = ModbusTcpClient::connect("127.0.0.1", 1, DEFAULT_UNIT_ID).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_register_verifies_echo() {
        // A matching FC06 echo (address + value) must succeed.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            if n >= 7 {
                let mut frame = buf[0..7].to_vec(); // echo MBAP header incl. unit
                frame.extend_from_slice(&buf[7..n]); // echo the PDU as-is
                let _ = sock.write_all(&frame).await;
            }
        });
        let mut client = ModbusTcpClient::connect(&addr.ip().to_string(), addr.port(), DEFAULT_UNIT_ID).await.unwrap();
        client.write_register(42, 350).await.expect("matching echo must succeed");

        // A value-mismatched echo (PLC applied a different value) must fail closed.
        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr2 = listener2.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener2.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            if n >= 7 {
                let mut frame = buf[0..7].to_vec();
                frame.extend_from_slice(&[0x06, 0x00, 0x2A, 0x00, 0x00]); // echoes value 0, not 350
                let _ = sock.write_all(&frame).await;
            }
        });
        let mut client2 = ModbusTcpClient::connect(&addr2.ip().to_string(), addr2.port(), DEFAULT_UNIT_ID).await.unwrap();
        let err = client2.write_register(42, 350).await;
        assert!(err.is_err(), "value-mismatched echo must fail verification");
    }

    #[tokio::test]
    async fn test_truncated_fc03_response_does_not_panic() {
        // Regression: a short FC03 body (len 3 with byte_count 2) used to panic
        // on response[3]. It must be a clean Err instead.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            if n >= 2 {
                // MBAP: tx = echo, proto=0, len=4 (unit + 3-byte body), unit=1,
                // body = [FC03, byte_count=2, single data byte] — truncated.
                let frame = [buf[0], buf[1], 0x00, 0x00, 0x00, 0x04, 0x01, 0x03, 0x02, 0x00];
                let _ = sock.write_all(&frame).await;
            }
        });
        let mut client = ModbusTcpClient::connect(&addr.ip().to_string(), addr.port(), DEFAULT_UNIT_ID).await.unwrap();
        let result = client.read_register(0).await;
        assert!(result.is_err(), "truncated response must be an error, not a panic");
    }

    #[tokio::test]
    async fn test_wrong_function_code_rejected() {
        // A response echoing a different function code (e.g. FC04) must be rejected.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 16];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            if n >= 2 {
                // Correct length, but function code 0x04 instead of 0x03.
                let frame = [buf[0], buf[1], 0x00, 0x00, 0x00, 0x05, 0x01, 0x04, 0x02, 0x00, 0x64];
                let _ = sock.write_all(&frame).await;
            }
        });
        let mut client = ModbusTcpClient::connect(&addr.ip().to_string(), addr.port(), DEFAULT_UNIT_ID).await.unwrap();
        let result = client.read_register(0).await;
        assert!(result.is_err(), "wrong function code must be rejected");
    }

    #[test]
    fn test_modbus_rtu_crc16() {
        // Known standard Modbus RTU test vector:
        // Request: 01 03 00 00 00 01 -> CRC = 84 0A (low byte 0x84, high byte 0x0A)
        let frame = [0x01, 0x03, 0x00, 0x00, 0x00, 0x01];
        let crc = calculate_crc16(&frame);
        assert_eq!(crc, 0x0A84);
    }

    #[test]
    fn test_modbus_rtu_frame_building() {
        let read_frame = ModbusRtuCodec::build_read_holding_registers(1, 0x0000, 1);
        assert_eq!(read_frame, vec![0x01, 0x03, 0x00, 0x00, 0x00, 0x01, 0x84, 0x0A]);

        let coil_on = ModbusRtuCodec::build_write_single_coil(1, 0x000A, true);
        assert_eq!(coil_on.len(), 8);
        assert_eq!(&coil_on[..6], &[0x01, 0x05, 0x00, 0x0A, 0xFF, 0x00]);
        let coil_crc = calculate_crc16(&coil_on[..6]);
        assert_eq!(&coil_on[6..], &coil_crc.to_le_bytes());

        let reg_write = ModbusRtuCodec::build_write_single_register(1, 0x0001, 500);
        assert_eq!(reg_write.len(), 8);
        assert_eq!(&reg_write[..6], &[0x01, 0x06, 0x00, 0x01, 0x01, 0xF4]);
    }

    #[test]
    fn test_modbus_rtu_decode_frame() {
        // Valid echo frame
        let mut frame = vec![0x01, 0x03, 0x02, 0x00, 0x64];
        let crc = calculate_crc16(&frame);
        frame.extend_from_slice(&crc.to_le_bytes());

        let decoded = ModbusRtuCodec::decode_frame(1, &frame).expect("valid frame should decode");
        assert_eq!(decoded, vec![0x03, 0x02, 0x00, 0x64]);

        // Wrong unit ID
        let err = ModbusRtuCodec::decode_frame(2, &frame);
        assert!(matches!(err, Err(ModbusError::UnitMismatch { sent: 2, received: 1 })));

        // CRC corruption
        let mut corrupted = frame.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0xFF;
        let err_crc = ModbusRtuCodec::decode_frame(1, &corrupted);
        assert!(matches!(err_crc, Err(ModbusError::CrcMismatch { .. })));

        // Modbus Exception response (0x80 | FC)
        let mut exc_frame = vec![0x01, 0x83, 0x02]; // FC03 exception 0x02 (Illegal Data Address)
        let exc_crc = calculate_crc16(&exc_frame);
        exc_frame.extend_from_slice(&exc_crc.to_le_bytes());
        let err_exc = ModbusRtuCodec::decode_frame(1, &exc_frame);
        assert!(matches!(err_exc, Err(ModbusError::Exception(2))));
    }
}
