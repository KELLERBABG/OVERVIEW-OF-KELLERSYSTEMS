//! Lock-Free Single-Producer Single-Consumer (SPSC) Ring Buffer

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A cache-aligned, lock-free SPSC circular ring buffer for high-rate DAQ event streaming.
pub struct SpscRingBuffer<T: Default + Clone> {
    buffer: Vec<T>,
    capacity: usize,
    mask: usize,
    head: Arc<AtomicUsize>, // Written by producer
    tail: Arc<AtomicUsize>, // Written by consumer
}

impl<T: Default + Clone> SpscRingBuffer<T> {
    /// Creates a ring buffer with power-of-two capacity.
    pub fn new(capacity_power_of_two: usize) -> Self {
        let capacity = capacity_power_of_two.next_power_of_two();
        let mask = capacity - 1;
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, Default::default);

        Self {
            buffer,
            capacity,
            mask,
            head: Arc::new(AtomicUsize::new(0)),
            tail: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Pushes an item into the buffer. Returns false if the buffer is full.
    #[inline]
    pub fn try_push(&mut self, item: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if (head.wrapping_sub(tail)) >= self.capacity {
            return false; // Buffer full
        }

        let idx = head & self.mask;
        self.buffer[idx] = item;
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Pops an item from the buffer. Returns None if the buffer is empty.
    #[inline]
    pub fn try_pop(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if head == tail {
            return None; // Buffer empty
        }

        let idx = tail & self.mask;
        let item = self.buffer[idx].clone();
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(item)
    }

    /// Returns the number of items currently waiting in the buffer.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
