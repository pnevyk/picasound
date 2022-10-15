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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FrameId(usize);

#[allow(clippy::new_without_default)]
impl FrameId {
    pub fn new() -> Self {
        Self(frame_id::get())
    }

    pub fn update(&mut self, other: Self) -> bool {
        if self.0 != other.0 {
            self.0 = other.0;
            true
        } else {
            false
        }
    }
}

mod frame_id {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Starting with 1 so it is not equal to the FrameId::default().
    static FRAME_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

    pub fn get() -> usize {
        FRAME_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}
