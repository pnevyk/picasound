use crate::{
    options::{Options, Value},
    pipeline::{node_ref, Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        inputs::validate_inputs,
        spectrum::{Spectrum, SpectrumStore, Stft, Window},
        video::VideoConfig,
        Error, FrameId,
    },
};

pub struct SpectrumNode {
    input: NodeRef,
    spectrum: SpectrumStore,
}

impl SpectrumNode {
    pub fn new(inputs: Vec<NodeRef>, options: Options) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideAudioData)?;

        let window_len = options
            .get("window-size")
            .unwrap_or(&4096.into())
            .as_i32()
            .ok_or(Error::InvalidOptions)? as usize;

        let default_frequency_range = (16.35, 7902.13).into();
        let frequency_range = options
            .get("frequency-range")
            // https://en.wikipedia.org/wiki/Pitch_(music)#Labeling_pitches
            .unwrap_or(&default_frequency_range)
            .as_slice()
            .ok_or(Error::InvalidOptions)?;

        let frequency_range = match frequency_range {
            [Value::Number(f_min), Value::Number(f_max)] if f_min < f_max => (*f_min, *f_max),
            _ => return Err(Error::InvalidOptions),
        };

        let spectrum = SpectrumStore::with_frequency_range(
            Stft::new(window_len, Window::Hann),
            frequency_range,
        );

        Ok(Self { input, spectrum })
    }
}

impl Node for SpectrumNode {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideSpectrum)
    }

    fn provide_spectrum(&self, id: FrameId) -> Spectrum {
        let data = self.input.provide_audio_data(id);
        self.spectrum.compute(
            id,
            &data.exact(self.spectrum.window_len()),
            data.sample_rate(),
        )
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "spectrum"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        options: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        SpectrumNode::new(inputs, options).map(node_ref)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
