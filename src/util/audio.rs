use std::{
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard},
};

pub const BUFFER_FRAMES: usize = 250;

#[derive(Debug, Clone)]
pub struct AudioBuffer {
    buf: Arc<Mutex<Vec<f32>>>,
    frame_size: usize,
    sample_rate: usize,
    buf_size: usize,
}

impl AudioBuffer {
    pub fn new(sample_rate: usize, fps: usize) -> Self {
        let frame_size = sample_rate / fps;
        let buf_size = frame_size * BUFFER_FRAMES;
        let buf = Arc::new(Mutex::new(Vec::with_capacity(2 * buf_size)));

        Self {
            buf,
            frame_size,
            sample_rate,
            buf_size,
        }
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn push(&self, data: &[f32]) {
        assert!(data.len() <= self.buf_size, "unexpectedly large data chunk");

        let mut buf = self.buf.lock().unwrap();
        let buf_len = buf.len();

        if buf_len + data.len() > 2 * self.buf_size {
            let new_head = buf_len - self.buf_size;
            buf.copy_within(new_head.., 0);
            buf.resize(self.buf_size, 0.0);
        }

        buf.extend_from_slice(data);
    }

    pub fn frames(&self, frames: usize) -> AudioDataGuard {
        let buf = self.buf.lock().unwrap();
        let len = (self.frame_size * frames).min(self.buf_size);
        let head = buf.len().saturating_sub(len);
        AudioDataGuard { buf, head }
    }

    pub fn exact(&self, n: usize) -> AudioDataGuard {
        let buf = self.buf.lock().unwrap();
        let len = n.min(self.buf_size);
        let head = buf.len().saturating_sub(len);
        AudioDataGuard { buf, head }
    }
}

#[derive(Debug)]
pub struct AudioDataGuard<'a> {
    buf: MutexGuard<'a, Vec<f32>>,
    head: usize,
}

impl<'a> Deref for AudioDataGuard<'a> {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        &self.buf[self.head..]
    }
}
