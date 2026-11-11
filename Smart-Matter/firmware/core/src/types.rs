use bitflags::bitflags;
use heapless::Vec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum payload size for a single protocol slice (bytes).
/// Chosen to fit within a single sub-millisecond energy window transmission
/// over 802.15.4 / BLE at the lowest power setting.
pub const MAX_PAYLOAD_SIZE: usize = 128;

/// Maximum number of fragments in a dead-drop storage message.
pub const MAX_FRAGMENTS: usize = 32;

/// Maximum number of hops for mesh routing.
pub const MAX_HOPS: u8 = 64;

/// Maximum node ID length (bytes).
pub const NODE_ID_LEN: usize = 8;

/// Maximum number of peers in a mesh routing table.
pub const MAX_PEERS: usize = 32;

/// Default TTL for dead-drop fragments (seconds).
pub const DEFAULT_DROP_TTL: u32 = 86400; // 24 hours

// ---------------------------------------------------------------------------
// Core Identifiers
// ---------------------------------------------------------------------------

/// A 64-bit node identifier. Unique per Smart Matter compute node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub [u8; NODE_ID_LEN]);

impl NodeId {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == NODE_ID_LEN {
            let mut id = [0u8; NODE_ID_LEN];
            id.copy_from_slice(bytes);
            Some(Self(id))
        } else {
            None
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Crystalline Resonance / Energy Timestamp
// ---------------------------------------------------------------------------

/// A compact time representation (milliseconds since epoch, 48-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TimeMs(pub u64);

/// Clock used by [`TimeMs::now`] on targets without `std`.
///
/// Embedded firmware updates this from its SysTick/RTC via
/// [`TimeMs::set_embedded_clock`]; it defaults to 0 until then.
///
/// Stored as `u32` milliseconds because Cortex-M0+ (thumbv6m) has no 64-bit
/// atomics; the value wraps every ~49.7 days, which is acceptable for TTLs
/// and check intervals in the 24 h range.
static EMBEDDED_CLOCK_MS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

impl TimeMs {
    pub fn now() -> Self {
        // On std this is wall-clock time; on embedded targets it is the
        // value last provided via `set_embedded_clock`.
        Self(now_ms())
    }

    /// Set the firmware clock (embedded targets). Call from the SysTick
    /// interrupt or RTC update so `TimeMs::now()` tracks real time.
    pub fn set_embedded_clock(ms: u64) {
        EMBEDDED_CLOCK_MS.store(ms as u32, core::sync::atomic::Ordering::Relaxed);
    }

    /// Current firmware clock value in ms (embedded targets).
    pub fn embedded_clock() -> u64 {
        EMBEDDED_CLOCK_MS.load(core::sync::atomic::Ordering::Relaxed) as u64
    }
}

#[cfg(feature = "std")]
fn now_ms() -> u64 {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    d.as_millis() as u64
}

#[cfg(not(feature = "std"))]
fn now_ms() -> u64 {
    EMBEDDED_CLOCK_MS.load(core::sync::atomic::Ordering::Relaxed) as u64
}

// ---------------------------------------------------------------------------
// Energy & Power
// ---------------------------------------------------------------------------

/// Energy level classification for node power budgeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyLevel {
    /// Node is actively harvesting and has spare capacity.
    Surplus,
    /// Normal operational range.
    Nominal,
    /// Low energy — reduce non-essential operations.
    Low,
    /// Critical — only keep-alive and wake-on-vibration.
    Critical,
}

/// Voltage reading from the piezoelectric harvester + supercapacitor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VoltageReading {
    /// Harvester output voltage (mV).
    pub harvester_mv: u32,
    /// Storage capacitor voltage (mV).
    pub storage_mv: u32,
    /// Regulated output voltage (mV).
    pub vdd_mv: u32,
}

// ---------------------------------------------------------------------------
// Sensor Data
// ---------------------------------------------------------------------------

/// A single sample from the onboard accelerometer / vibration sensor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VibrationSample {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    /// Timestamp of the sample.
    pub time: TimeMs,
}

/// Environmental entropy sample from ADC noise + sensor data.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntropySample {
    /// Raw ADC noise reading (differential channel).
    pub adc_noise: u32,
    /// Accelerometer noise floor.
    pub vibration_floor: u16,
    /// Temperature at time of sample (millidegrees Celsius).
    pub temperature_mc: i32,
}

// ---------------------------------------------------------------------------
// Protocol Slicing
// ---------------------------------------------------------------------------

/// Identifies the protocol slice type within a transmission window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SliceType {
    /// Beacon / presence announcement.
    Beacon = 0x01,
    /// Mesh routing advertisement.
    RouteAdv = 0x02,
    /// Data fragment (dead-drop storage).
    DataFragment = 0x03,
    /// Data fragment acknowledgment.
    DataAck = 0x04,
    /// Integrity check / health report.
    IntegrityReport = 0x05,
    /// Broadcast message (localized).
    Broadcast = 0x06,
    /// Configuration / command from desktop tool.
    Command = 0x07,
    /// Firmware update slice.
    FwUpdate = 0x08,
}

/// A single transmission slice — fits within one sub-ms energy window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolSlice {
    /// The type of this slice.
    pub slice_type: SliceType,
    /// Source node ID.
    pub src: NodeId,
    /// Destination node ID (or broadcast).
    pub dst: NodeId,
    /// Hop count (incremented each relay).
    pub hops: u8,
    /// Sequence number for deduplication.
    pub seqno: u16,
    /// Payload (max 128 bytes).
    pub payload: Vec<u8, MAX_PAYLOAD_SIZE>,
    /// Energy level of the sender at transmission.
    pub energy: EnergyLevel,
}

// ---------------------------------------------------------------------------
// Mesh Routing
// ---------------------------------------------------------------------------

bitflags! {
    /// Flags for mesh routing advertisements.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RouteFlags: u8 {
        /// Node is a sink/gateway.
        const IS_SINK     = 0b0001;
        /// Node accepts new peers.
        const ACCEPT_PEER = 0b0010;
        /// Node has storage available.
        const HAS_STORAGE = 0b0100;
        /// Node is in low-power duty cycle.
        const LOW_POWER   = 0b1000;
    }
}

/// A single routing table entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub destination: NodeId,
    pub next_hop: NodeId,
    pub metric: u16, // Gradient metric (lower is better).
    pub flags: RouteFlags,
    pub age_sec: u32, // Seconds since last update.
}

/// A mesh routing advertisement packet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteAdvertisement {
    pub origin: NodeId,
    pub seqno: u16,
    pub flags: RouteFlags,
    pub entries: Vec<RouteEntry, MAX_PEERS>,
}

// ---------------------------------------------------------------------------
// Dead-Drop Storage
// ---------------------------------------------------------------------------

/// Fragment header for dead-drop storage messages.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FragmentHeader {
    /// Message ID (all fragments share this).
    pub msg_id: u32,
    /// Total number of fragments in this message.
    pub total: u8,
    /// This fragment index (0-based).
    pub index: u8,
    /// Time-to-live in seconds from creation.
    pub ttl_sec: u32,
    /// Creation timestamp.
    pub created: TimeMs,
}

/// A single encrypted dead-drop fragment stored on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadDropFragment {
    pub header: FragmentHeader,
    /// Encrypted payload (ChaCha20-Poly1305) — ciphertext + 16-byte tag.
    pub ciphertext: Vec<u8, { MAX_PAYLOAD_SIZE + 16 }>,
    /// Nonce for this fragment.
    pub nonce: [u8; 12],
}

// ---------------------------------------------------------------------------
// Broadcast
// ---------------------------------------------------------------------------

/// Localized broadcast message with RSSI-based geo-fencing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub origin: NodeId,
    /// Maximum RSSI threshold (dBm) — nodes with RSSI below this
    /// will not relay the message, enforcing geographical locality.
    pub rssi_threshold: i16,
    pub seqno: u16,
    pub payload: Vec<u8, MAX_PAYLOAD_SIZE>,
    /// Time-to-live in mesh hops.
    pub ttl: u8,
}

// ---------------------------------------------------------------------------
// Thermal & Integrity
// ---------------------------------------------------------------------------

/// Thermal state of a compute node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermalState {
    /// Core temperature (millidegrees Celsius).
    pub core_mc: i32,
    /// Ambient temperature estimate (millidegrees Celsius).
    pub ambient_mc: i32,
    /// Thermal capacity utilization (0-100%).
    pub capacity_pct: u8,
    /// Recommended duty cycle (0-100%) based on thermal model.
    pub recommended_duty_pct: u8,
}

/// Structural integrity assessment from vibration analysis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub node_id: NodeId,
    /// Baseline vibration amplitude (μm).
    pub baseline_amplitude_um: u16,
    /// Current vibration amplitude (μm).
    pub current_amplitude_um: u16,
    /// Deviation from baseline as percentage (0-100).
    pub deviation_pct: u8,
    /// Number of anomaly events detected.
    pub anomaly_count: u32,
    /// Confidence of anomaly detection (0-255).
    pub anomaly_confidence: u8,
}

// ---------------------------------------------------------------------------
// Command / Configuration
// ---------------------------------------------------------------------------

/// Commands from the desktop configuration tool to a node.
///
/// `FwData` carries a full 128-byte slice payload, which makes this the
/// largest variant by far; the enum is only ever constructed/destructured
/// one variant at a time on the stack, so the size difference is harmless.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeCommand {
    /// Reboot the node.
    Reboot,
    /// Enter low-power sleep.
    Sleep { duration_ms: u32 },
    /// Set the RSSI threshold for broadcast geo-fencing.
    SetRssiThreshold(i16),
    /// Set the dead-drop TTL (seconds).
    SetDropTtl(u32),
    /// Update routing table.
    UpdateRoutes(Vec<RouteEntry, MAX_PEERS>),
    /// Start firmware update.
    StartFwUpdate {
        total_slices: u16,
        checksum: [u8; 32],
    },
    /// Firmware update data slice.
    FwData {
        slice_index: u16,
        data: Vec<u8, 128>,
    },
    /// Request integrity report.
    RequestIntegrityReport,
    /// Set duty cycle.
    SetDutyCycle(u8),
}

/// Response from a node to a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeResponse {
    Ack,
    Nak { reason: &'static str },
    IntegrityReport(IntegrityReport),
    ThermalState(ThermalState),
    EnergyStatus(VoltageReading),
}
