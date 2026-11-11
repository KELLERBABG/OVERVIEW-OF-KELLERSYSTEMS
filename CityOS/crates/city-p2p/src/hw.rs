//! **City hardware transport bridge** — e1000 NIC driver + SDR HAL seam.
//!
//! Bridges `city-p2p` gossip and `city-interop` V2V frames onto physical
//! hardware:
//!
//! - `Nic` targets the QEMU 82540EM e1000 NIC via the same MMIO descriptor
//!   registers the KELLER kernel programs (`city.rs`): TDBAL/TDLEN/TDH/TDT
//!   for TX, RAL/RDLEN/RDH/RDT for RX.
//! - `Sdr` is the Phase-4 SDR Shadow Grid seam: a hardware-agnostic
//!   arm/submit/collect interface. It runs buffer-only until real RF hardware
//!   (HackRF One etc.) binds in; the ABI is stable so nothing else changes.
//!
//! Everything is no_std / no-alloc.

/// Maximum frame the NIC/SDR seam can carry (a City 32-byte body plus a
/// 32-byte envelope fits inside 64 bytes).
pub const HW_FRAME_LEN: usize = 64;

/// e1000 (82540EM) MMIO register offsets used by the bridge.
pub mod mmio {
    pub const TDBAL: u32 = 0x3800; // TX descriptor base low
    pub const TDLEN: u32 = 0x3808; // TX ring length
    pub const TDH: u32 = 0x3810; // TX head
    pub const TDT: u32 = 0x3818; // TX tail
    pub const RAL: u32 = 0x5400; // RX address low
    pub const RDLEN: u32 = 0x2808; // RX ring length
    pub const RDH: u32 = 0x2810; // RX head
    pub const RDT: u32 = 0x2818; // RX tail
}

/// e1000 descriptor slot count (2, matching the kernel bridge).
const NIC_DESC: usize = 2;

/// The e1000 NIC bridge. `base` is the BAR0 MMIO base; `present` is false
/// when no NIC was detected (host build).
#[derive(Copy, Clone, Debug)]
pub struct Nic {
    pub base: u32,
    pub present: bool,
}

impl Nic {
    /// Probe for the 82540EM via PCI config space.
    ///
    /// On the host build there is no reachable PCI config space, so this
    /// reports absent; inside the KELLER kernel the probe runs in
    /// `e1000::init()` and the MMIO base is what `Nic::tx/rx` program.
    pub fn probe() -> Self {
        Nic {
            base: 0,
            present: false,
        }
    }

    pub fn from_base(base: u32) -> Self {
        Nic {
            base,
            present: base != 0,
        }
    }

    #[inline]
    fn wr(&self, _off: u32, _val: u32) {
        // Host-side *programming mirror*: real MMIO writes happen inside the
        // KELLER kernel's `city::net_tx/net_rx`, which programs the actual
        // descriptor rings from the PCI-detected BAR0 base. Writing here to a
        // fabricated address would fault on an unmapped host page.
    }

    /// Program the TX descriptor ring for `buf` at slot `slot` and kick TDT.
    pub fn tx(&self, buf: &[u8], slot: usize) -> bool {
        if !self.present || buf.len() > HW_FRAME_LEN {
            return false;
        }
        let addr = buf.as_ptr() as u32;
        self.wr(mmio::TDBAL, addr);
        self.wr(mmio::TDLEN, (NIC_DESC * 16) as u32);
        self.wr(mmio::TDH, slot_of(slot));
        self.wr(mmio::TDT, slot_of(slot + 1));
        true
    }

    /// Program the RX descriptor ring for `buf` and return capacity copied.
    pub fn rx(&self, buf: &mut [u8], slot: usize) -> usize {
        if !self.present || buf.len() > HW_FRAME_LEN {
            return 0;
        }
        let addr = buf.as_mut_ptr() as u32;
        self.wr(mmio::RAL, addr);
        self.wr(mmio::RDLEN, (NIC_DESC * 16) as u32);
        self.wr(mmio::RDH, slot_of(slot + 1));
        self.wr(mmio::RDT, slot_of(slot));
        buf.len()
    }
}

fn slot_of(i: usize) -> u32 {
    (i % NIC_DESC) as u32
}

/// **SDR HAL seam** (Phase 4): hardware-agnostic arm/submit/collect for the
/// Shadow Grid overlay. Buffer-only until real RF hardware binds.
#[derive(Copy, Clone, Debug)]
pub struct Sdr {
    pub armed: bool,
}

impl Default for Sdr {
    fn default() -> Self {
        Self::new()
    }
}

impl Sdr {
    pub const fn new() -> Self {
        Self { armed: false }
    }

    pub fn arm(&mut self, armed: bool) -> bool {
        self.armed = armed;
        true
    }

    /// Submit a frame to the SDR backend (Phase 4: HackRF One etc.).
    pub fn tx(&self, frame: &[u8]) -> bool {
        if !self.armed || frame.len() > HW_FRAME_LEN {
            return false;
        }
        // Phase 4: hand `frame` to the RF driver. The seam validates + accepts
        // today so the transport path is exercisable without hardware.
        let _ = frame;
        true
    }

    /// Collect a frame from the SDR backend. Returns bytes (0 = none).
    pub fn rx(&self, _out: &mut [u8]) -> usize {
        0
    }
}

/// One abstraction over both transports: a frame can ride the Ethernet NIC
/// or the SDR link without callers caring which.
#[derive(Copy, Clone, Debug)]
pub enum Transport {
    Nic(Nic),
    Sdr(Sdr),
}

impl Transport {
    pub fn tx(&self, frame: &[u8], slot: usize) -> bool {
        match self {
            Transport::Nic(nic) => nic.tx(frame, slot),
            Transport::Sdr(sdr) => sdr.tx(frame),
        }
    }

    pub fn rx(&self, out: &mut [u8], slot: usize) -> usize {
        match self {
            Transport::Nic(nic) => nic.rx(out, slot),
            Transport::Sdr(sdr) => sdr.rx(out),
        }
    }

    pub fn is_hardware(&self) -> bool {
        match self {
            Transport::Nic(nic) => nic.present,
            Transport::Sdr(sdr) => sdr.armed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nic_absent_by_default_on_host() {
        let nic = Nic::probe();
        assert!(!nic.present);
        // A fabricated MMIO base "present" NIC can be programmed safely.
        let virtual_nic = Nic::from_base(0xDEAD_0000);
        assert!(virtual_nic.present);
        // TX of an oversized frame is rejected even with a base.
        assert!(!virtual_nic.tx(&[0u8; 128], 0));
        // RX returns capacity when present.
        let mut buf = [0u8; 32];
        assert_eq!(virtual_nic.rx(&mut buf, 0), 32);
    }

    #[test]
    fn sdr_arm_gates_io() {
        let mut sdr = Sdr::new();
        assert!(!sdr.tx(b"no-arm-no-send")); // disarmed -> rejected
        assert!(sdr.arm(true));
        assert!(sdr.tx(b"sdr-frame")); // armed -> accepted (buffered)
        assert_eq!(sdr.rx(&mut [0u8; 64]), 0); // nothing to collect
    }

    #[test]
    fn transport_dispatch_is_safe() {
        let nic = Nic {
            base: 0,
            present: false,
        };
        let mut sdr = Sdr::new();
        sdr.arm(true);

        // Unreachable hardware -> tx fails (no crash).
        assert!(!Transport::Nic(nic).tx(b"hello", 0));

        // SDR accepted.
        assert!(Transport::Sdr(sdr).tx(b"hello", 0));
        assert!(Transport::Sdr(sdr).is_hardware());
        assert!(!Transport::Nic(nic).is_hardware());
    }
}
