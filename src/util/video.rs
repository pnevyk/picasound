pub struct Pixel<'a> {
    // BGRx pixel encoding
    buf: &'a mut [u8],
}

impl<'a> Pixel<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        debug_assert!(buf.len() >= 4);
        Self { buf }
    }

    pub fn red(&self) -> u8 {
        self.buf[2]
    }

    pub fn green(&self) -> u8 {
        self.buf[1]
    }

    pub fn blue(&self) -> u8 {
        self.buf[0]
    }

    pub fn red_f(&self) -> f32 {
        to_f(self.red())
    }

    pub fn green_f(&self) -> f32 {
        to_f(self.green())
    }

    pub fn blue_f(&self) -> f32 {
        to_f(self.blue())
    }

    pub fn set_red(&mut self, value: u8) {
        self.buf[2] = value;
    }

    pub fn set_green(&mut self, value: u8) {
        self.buf[1] = value;
    }

    pub fn set_blue(&mut self, value: u8) {
        self.buf[0] = value;
    }

    pub fn set_red_f(&mut self, value: f32) {
        self.set_red(from_f(value));
    }

    pub fn set_green_f(&mut self, value: f32) {
        self.set_green(from_f(value));
    }

    pub fn set_blue_f(&mut self, value: f32) {
        self.set_blue(from_f(value));
    }

    pub fn set_grayscale(&mut self, value: u8) {
        self.buf[..3].fill(value);
    }

    pub fn set_grayscale_f(&mut self, value: f32) {
        self.set_grayscale(from_f(value));
    }
}

fn to_f(byte: u8) -> f32 {
    byte as f32 / 255.0
}

fn from_f(value: f32) -> u8 {
    (value * 255.0) as u8
}

impl<'a> std::fmt::Debug for Pixel<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelMut")
            .field("red", &self.red())
            .field("green", &self.green())
            .field("blue", &self.blue())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    buf: Vec<u8>,
    width: usize,
    height: usize,
    stride: usize,
}

impl VideoFrame {
    pub fn new(width: usize, height: usize) -> Self {
        // BGRx pixel encoding;
        let stride = width * 4;
        let buf = vec![0; stride * height];

        Self {
            buf,
            width,
            height,
            stride,
        }
    }

    pub fn buf(&self) -> &[u8] {
        &self.buf
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn stride(&self) -> usize {
        self.stride
    }

    pub fn copy_from(&mut self, other: &Self) {
        assert_eq!(self.width, other.width);
        assert_eq!(self.height, other.height);
        self.buf.copy_from_slice(&other.buf);
    }

    pub fn clear(&mut self) {
        self.buf.fill(0);
    }

    pub fn apply<F>(&mut self, mut apply: F)
    where
        F: FnMut((usize, usize), &mut Pixel),
    {
        for (y, line) in self.buf.chunks_exact_mut(self.stride).enumerate() {
            for (x, pixel) in line[..(4 * self.width)].chunks_exact_mut(4).enumerate() {
                apply((x, y), &mut Pixel::new(pixel))
            }
        }
    }

    pub fn apply_zip<F>(&mut self, other: &mut Self, mut apply: F)
    where
        F: FnMut((usize, usize), &mut Pixel, &mut Pixel),
    {
        assert_eq!(self.width, other.width);
        assert_eq!(self.height, other.height);

        for (y, (line1, line2)) in self
            .buf
            .chunks_exact_mut(self.stride)
            .zip(other.buf.chunks_exact_mut(other.stride))
            .enumerate()
        {
            for (x, (pixel1, pixel2)) in line1[..(4 * self.width)]
                .chunks_exact_mut(4)
                .zip(line2[..(4 * other.width)].chunks_exact_mut(4))
                .enumerate()
            {
                apply((x, y), &mut Pixel::new(pixel1), &mut Pixel::new(pixel2))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoConfig {
    width: usize,
    height: usize,
    fps: usize,
}

impl VideoConfig {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn fps(&self) -> usize {
        self.fps
    }

    pub fn builder() -> VideoConfigBuilder {
        VideoConfigBuilder {
            config: VideoConfig::default(),
        }
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 24,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VideoConfigBuilder {
    config: VideoConfig,
}

impl VideoConfigBuilder {
    pub fn width(&mut self, value: usize) -> &mut Self {
        assert!((352..=7680).contains(&value), "invalid range");
        assert!(value % 2 == 0, "odd value");
        self.config.width = value;
        self
    }

    pub fn height(&mut self, value: usize) -> &mut Self {
        assert!((240..=4320).contains(&value), "invalid range");
        assert!(value % 2 == 0, "odd value");
        self.config.height = value;
        self
    }

    pub fn fps(&mut self, value: usize) -> &mut Self {
        assert!((1..=60).contains(&value), "invalid range");
        self.config.fps = value;
        self
    }

    pub fn build(&self) -> VideoConfig {
        self.config
    }
}
