use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(AtomicU32::new(value.to_bits()))
    }

    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.0.load(order))
    }

    pub fn store(&self, value: f32, order: Ordering) {
        self.0.store(value.to_bits(), order);
    }
}

#[derive(Debug)]
pub struct FrameCounter {
    counter: AtomicUsize,
    modulo: usize,
}

impl FrameCounter {
    pub fn new(modulo: usize) -> Self {
        assert!(modulo > 0);
        Self {
            counter: AtomicUsize::new(0),
            modulo,
        }
    }

    pub fn fetch_inc(&self, order: Ordering) -> usize {
        let value = self.counter.fetch_add(1, order);

        if value + 1 == self.modulo {
            self.counter.store(0, order);
        }

        value
    }
}
