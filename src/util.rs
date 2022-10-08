use std::sync::atomic::{AtomicUsize, Ordering};

pub mod audio;
pub mod inputs;
pub mod misc;
pub mod spectrum;
pub mod video;

#[derive(Debug, Clone)]
pub enum Error {
    System,
    InvalidInputs,
    InvalidOptions,
    UnknownNode(String),
    InvalidPipeline(InvalidPipeline),
}

#[derive(Debug, Clone)]
pub enum InvalidPipeline {
    SourceProcessorsSinkOrder,
    Cycle,
    UnknownInput,
    InvalidSyntax,
}

// Can be used for caching where appropriate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(usize);

#[allow(clippy::new_without_default)]
impl FrameId {
    pub fn new() -> Self {
        Self(frame_id::get())
    }
}

#[derive(Debug)]
pub struct LastFrameId(AtomicUsize);

impl LastFrameId {
    pub fn store_if_not_eq(&self, id: FrameId) -> bool {
        let last = self.0.load(Ordering::Relaxed);
        if last != id.0 {
            self.0.store(id.0, Ordering::Relaxed);
            true
        } else {
            false
        }
    }
}

impl Default for LastFrameId {
    fn default() -> Self {
        // We must not use zero, because that is also the default value for
        // FrameId. If this default was zero, the cache hit on the first frame
        // would be false positive.
        Self(AtomicUsize::new(usize::MAX))
    }
}

mod frame_id {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static FRAME_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub fn get() -> usize {
        FRAME_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}
