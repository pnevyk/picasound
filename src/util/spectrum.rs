use std::{
    fmt,
    ops::Deref,
    sync::{Arc, Mutex},
};

use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};

use super::{FrameId, LastFrameId};

pub struct SpectrumStore {
    spectrum: Arc<Mutex<dyn ComputeSpectrum + Send + Sync>>,
    window_len: usize,
    last_id: LastFrameId,
    frequency_range: Option<(f32, f32)>,
}

impl SpectrumStore {
    pub fn new<S>(spectrum: S) -> Self
    where
        S: ComputeSpectrum + Send + Sync + 'static,
    {
        Self::new_inner(spectrum, None)
    }

    pub fn with_frequency_range<S>(spectrum: S, frequency_range: (f32, f32)) -> Self
    where
        S: ComputeSpectrum + Send + Sync + 'static,
    {
        Self::new_inner(spectrum, Some(frequency_range))
    }

    fn new_inner<S>(spectrum: S, frequency_range: Option<(f32, f32)>) -> Self
    where
        S: ComputeSpectrum + Send + Sync + 'static,
    {
        if let Some((f_min, f_max)) = frequency_range {
            assert!(f_min < f_max);
            assert!(f_min > 0.0);
        }

        let window_len = spectrum.window_len();
        Self {
            spectrum: Arc::new(Mutex::new(spectrum)),
            window_len,
            last_id: LastFrameId::default(),
            frequency_range,
        }
    }

    pub fn compute(&self, id: FrameId, data: &[f32], sample_rate: usize) -> Spectrum {
        let mut guard = self.spectrum.lock().unwrap();
        let spectrum = if self.last_id.store_if_not_eq(id) {
            guard.compute(data)
        } else {
            guard.get()
        };

        let orig_len = spectrum.len();
        let (spectrum, bin0, f_min, f_max) = if let Some((f_min, f_max)) = self.frequency_range {
            let bin0 = bin_for(orig_len, f_min, sample_rate);
            let spectrum = &spectrum[bin0..bin_for(orig_len, f_max, sample_rate)];
            (spectrum, bin0, f_min, f_max)
        } else {
            (spectrum, 0, 0.0, f32::INFINITY)
        };

        Spectrum {
            spectrum: spectrum.to_vec(),
            bin0,
            orig_len,
            frequency_range: (f_min, f_max),
        }
    }

    pub fn window_len(&self) -> usize {
        self.window_len
    }
}

impl fmt::Debug for SpectrumStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpectrumStore")
            .field("spectrum", &self.spectrum.lock().unwrap().name())
            .field("window_len", &self.window_len)
            .field("last_id", &self.last_id)
            .field("frequency_range", &self.frequency_range)
            .finish()
    }
}

pub struct Spectrum {
    spectrum: Vec<Complex<f32>>,
    bin0: usize,
    orig_len: usize,
    frequency_range: (f32, f32),
}

impl Spectrum {
    pub fn bin_for(&self, f: f32, sample_rate: usize) -> usize {
        let (f_min, f_max) = self.frequency_range;
        assert!((f_min..=f_max).contains(&f));
        bin_for(self.orig_len, f, sample_rate) - self.bin0
    }

    pub fn freq(&self, bin: usize, sample_rate: usize) -> f32 {
        freq(self.orig_len, self.bin0 + bin, sample_rate)
    }
}

impl Deref for Spectrum {
    type Target = [Complex<f32>];

    fn deref(&self) -> &Self::Target {
        &self.spectrum
    }
}

fn bin_for(full_spectrum_size: usize, f: f32, sample_rate: usize) -> usize {
    let n = full_spectrum_size as f32;
    let sr = sample_rate as f32;
    (f * n / sr).round() as usize
}

fn freq(full_spectrum_size: usize, bin: usize, sample_rate: usize) -> f32 {
    (bin as f32) * (sample_rate as f32) / (full_spectrum_size as f32)
}

pub trait ComputeSpectrum {
    fn name(&self) -> &'static str;
    fn window_len(&self) -> usize;
    fn compute(&mut self, data: &[f32]) -> &[Complex<f32>];
    fn get(&self) -> &[Complex<f32>];
}

pub struct Stft {
    processor: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    output: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    window: Window,
}

impl Stft {
    pub fn new(window_len: usize, window: Window) -> Self {
        let mut planner = RealFftPlanner::new();
        let processor = planner.plan_fft_forward(window_len);

        let input = processor.make_input_vec();
        let output = processor.make_output_vec();
        let scratch = processor.make_scratch_vec();

        debug_assert_eq!(input.len(), window_len);

        Self {
            processor,
            input,
            output,
            scratch,
            window,
        }
    }
}

impl ComputeSpectrum for Stft {
    fn name(&self) -> &'static str {
        "stft"
    }

    fn window_len(&self) -> usize {
        self.input.len()
    }

    fn compute(&mut self, data: &[f32]) -> &[Complex<f32>] {
        assert_eq!(self.input.len(), data.len(), "invalid window size");
        self.input.copy_from_slice(data);

        self.window.apply(&mut self.input);

        self.processor
            .process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch)
            .expect("valid inputs");

        let normalization = (self.input.len() as f32).sqrt();
        self.output.iter_mut().for_each(|x| *x /= normalization);

        self.get()
    }

    fn get(&self) -> &[Complex<f32>] {
        &self.output
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Window {
    Hann,
}

impl Window {
    pub fn apply(&self, data: &mut [f32]) {
        let len = data.len() as f32;
        match self {
            Window::Hann => data
                .iter_mut()
                .enumerate()
                .for_each(|(n, x)| *x *= windows::hann(n as f32, len)),
        }
    }
}

mod windows {
    pub fn hann(n: f32, len: f32) -> f32 {
        0.54 - 0.46 * (2.0 * std::f32::consts::PI * n / len).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window_sanity_check(window: Window) {
        let n = 32;
        let mut data = vec![1.0; n];
        window.apply(&mut data);

        assert!(data[0] <= 0.1, "first small");
        assert!(data[n - 1] <= 0.1, "last small");
        assert!(data.iter().copied().any(|x| x == 1.0), "a peak in 1");
        assert!(
            data.iter().copied().all(|x| (0.0..=1.0).contains(&x)),
            "proper range"
        );
    }

    #[test]
    fn hann_sanity_check() {
        window_sanity_check(Window::Hann);
    }
}
