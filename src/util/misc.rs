#[derive(Debug)]
pub struct FrameCounter {
    counter: usize,
    modulo: usize,
}

impl FrameCounter {
    pub fn new(modulo: usize) -> Self {
        assert!(modulo > 0);
        Self { counter: 0, modulo }
    }

    pub fn fetch_inc(&mut self) -> usize {
        let value = self.counter;
        self.counter = (self.counter + 1) % self.modulo;
        value
    }
}
