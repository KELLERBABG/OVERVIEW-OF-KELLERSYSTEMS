//! City OS IPC channels.
//!
//! ALIGNED with KELLER OS (`sovereign-core/src/ipc.rs`): a fixed set of
//! channels, each a ring buffer of 32-byte messages with a per-channel
//! **secret key**, a keyed SipHash-2-4 MAC over (sender, type, seq,
//! payload), and strict anti-replay sequence verification. The receiver
//! drops any frame with a bad MAC or a replayed/out-of-order sequence
//! number.
//!
//! Wire frame (32 bytes): `payload[24] ‖ tag[8]`. The 64-bit tag is the
//! SipHash-2-4 output; the 24-byte payload is the City body that fits one
//! KELLER message.
//!
//! City OS layers its typed `CityMsg` envelope on top so services can share
//! the exact same transport the kernel uses for sandboxed drivers.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::mac::{city_mac, verify_mac, MacKey};
use city_types::{CityMsg, MSG_LEN};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

pub const IPC_MAX_CHANNELS: usize = 8;
pub const IPC_RING_LEN: usize = 16;
/// Domain label for the per-channel key derivation (host profile).
pub const MAC_LABEL: u32 = 0xA5C3_4B71;
/// Number of payload bytes in a 32-byte frame (24 payload + 8-byte MAC tag).
pub const IPC_PAYLOAD_LEN: usize = MSG_LEN - city_crypto::MAC_TAG_LEN;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// Per-channel secret MAC key (host profile: label-derived; kernel:
    /// vault-held). Installed by `init()` before any traffic flows.
    pub key: MacKey,
    pub next_send_seq: u32,
    pub next_recv_seq: u32,
    pub ring: [CityMsg; IPC_RING_LEN],
    pub head: usize,
    pub tail: usize,
    pub count: usize,
    pub active: bool,
}

impl Channel {
    pub const fn new(_id: usize) -> Self {
        Self {
            // Zero key until `init()` installs the real derived key.
            key: MacKey::ZERO,
            next_send_seq: 1,
            next_recv_seq: 1,
            ring: [CityMsg::new(0, 0); IPC_RING_LEN],
            head: 0,
            tail: 0,
            count: 0,
            active: false,
        }
    }

    /// The derived per-channel key (host profile).
    pub fn key_for(id: usize) -> MacKey {
        MacKey::derive(MAC_LABEL ^ (id as u32).wrapping_mul(0x9E37_79B9))
    }

    /// Send a CityMsg. Returns false if the ring is full or the body too big.
    pub fn send(&mut self, sender: u32, msg_type: u32, payload: &[u8]) -> bool {
        if self.count >= IPC_RING_LEN || payload.len() > IPC_PAYLOAD_LEN {
            return false;
        }
        // The wire body is a fixed 32-byte frame: 24 payload bytes, then the
        // 8-byte SipHash-2-4 tag. The MAC is computed over the 24-byte
        // payload so send and recv verify identically.
        let mut body = [0u8; MSG_LEN];
        body[..payload.len()].copy_from_slice(payload);
        let mac = city_mac(
            &self.key,
            sender,
            msg_type,
            self.next_send_seq,
            &body[..IPC_PAYLOAD_LEN],
        );
        body[IPC_PAYLOAD_LEN..].copy_from_slice(&mac.to_le_bytes());
        let mut m = CityMsg::new(sender, msg_type);
        m.body.0 = body;
        self.next_send_seq = self.next_send_seq.wrapping_add(1);
        self.ring[self.head] = m;
        self.head = (self.head + 1) % IPC_RING_LEN;
        self.count += 1;
        true
    }

    /// Receive the next verified message. Returns None on empty or tampered.
    pub fn recv(&mut self) -> Option<CityMsg> {
        if self.count == 0 {
            return None;
        }
        let m = self.ring[self.tail];
        // Verify MAC (anti-forgery) and sequence (anti-replay).
        let mac_field = u64::from_le_bytes([
            m.body.0[MSG_LEN - 8],
            m.body.0[MSG_LEN - 7],
            m.body.0[MSG_LEN - 6],
            m.body.0[MSG_LEN - 5],
            m.body.0[MSG_LEN - 4],
            m.body.0[MSG_LEN - 3],
            m.body.0[MSG_LEN - 2],
            m.body.0[MSG_LEN - 1],
        ]);
        let mut payload = [0u8; IPC_PAYLOAD_LEN];
        payload.copy_from_slice(&m.body.0[..IPC_PAYLOAD_LEN]);
        let ok = verify_mac(
            &self.key,
            m.sender,
            m.msg_type,
            self.next_recv_seq,
            &payload[..IPC_PAYLOAD_LEN],
            mac_field,
        );
        self.tail = (self.tail + 1) % IPC_RING_LEN;
        self.count -= 1;
        if !ok {
            return None;
        }
        self.next_recv_seq = self.next_recv_seq.wrapping_add(1);
        Some(m)
    }

    pub fn ready(&self) -> bool {
        self.count > 0
    }
}

/// Tiny spin-lock (same model as city-caps).
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}
impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}
impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.value.get() }
    }
}
impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() }
    }
}
impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

pub static CHANNELS: SpinLock<[Channel; IPC_MAX_CHANNELS]> = SpinLock::new([
    Channel::new(0),
    Channel::new(1),
    Channel::new(2),
    Channel::new(3),
    Channel::new(4),
    Channel::new(5),
    Channel::new(6),
    Channel::new(7),
]);

pub fn init() {
    let mut chs = CHANNELS.lock();
    for (id, ch) in chs.iter_mut().enumerate() {
        ch.key = Channel::key_for(id);
        ch.active = true;
        ch.next_send_seq = 1;
        ch.next_recv_seq = 1;
        ch.count = 0;
        ch.head = 0;
        ch.tail = 0;
    }
}

/// Send a City message on a channel. Returns false on full/oversize.
pub fn send(id: usize, sender: u32, msg_type: u32, payload: &[u8]) -> bool {
    if id >= IPC_MAX_CHANNELS {
        return false;
    }
    CHANNELS.lock()[id].send(sender, msg_type, payload)
}

/// Receive a verified City message from a channel.
pub fn recv(id: usize) -> Option<CityMsg> {
    if id >= IPC_MAX_CHANNELS {
        return None;
    }
    CHANNELS.lock()[id].recv()
}

/// Does the channel have a pending verified message?
pub fn ready(id: usize) -> bool {
    if id >= IPC_MAX_CHANNELS {
        return false;
    }
    CHANNELS.lock()[id].ready()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The channel table is a crate-level static shared by every IPC test;
    // serialize tests on the host so init()/send/recv don't race.
    static CHANNELS_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn roundtrip_verified() {
        let _g = CHANNELS_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        init();
        assert!(send(2, 7, 1, b"hello-city"));
        let m = recv(2).expect("message");
        assert_eq!(m.sender, 7);
        assert_eq!(m.msg_type, 1);
        // Payload bytes are preserved before the MAC tag.
        assert_eq!(&m.body.0[..10], b"hello-city");
    }

    #[test]
    fn tampered_mac_is_dropped() {
        let _g = CHANNELS_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        init();
        assert!(send(3, 1, 2, b"payload-p"));
        // Corrupt one payload byte in the ring head.
        {
            let mut chs = CHANNELS.lock();
            let ch = &mut chs[3];
            let head = (ch.head + IPC_RING_LEN - 1) % IPC_RING_LEN;
            ch.ring[head].body.0[0] ^= 0xFF;
        }
        assert!(recv(3).is_none()); // dropped, channel drains
    }

    #[test]
    fn replay_seq_is_rejected() {
        let _g = CHANNELS_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        init();
        assert!(send(4, 2, 3, b"seq-1"));
        let _ = recv(4).unwrap(); // seq 1 consumed
                                  // A forged frame reusing seq 1 must not pass.
        let mut chs = CHANNELS.lock();
        let ch = &mut chs[4];
        let mut forge = CityMsg::new(2, 3);
        let mut body = [0u8; MSG_LEN];
        body[..5].copy_from_slice(b"seq-1");
        forge.body.0 = body;
        // Forge with the same wire format the real channel uses: MAC over the
        // 24-byte frame, but with the *replayed* seq 1.
        let mac = city_mac(&ch.key, 2, 3, 1, &body[..IPC_PAYLOAD_LEN]);
        forge.body.0[IPC_PAYLOAD_LEN..].copy_from_slice(&mac.to_le_bytes());
        ch.ring[ch.head] = forge;
        ch.head = (ch.head + 1) % IPC_RING_LEN;
        ch.count += 1;
        drop(chs);
        assert!(recv(4).is_none()); // next expected seq is 2, so replay fails
    }
}
