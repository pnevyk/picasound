use crate::{
    options::Options,
    pipeline::{node_ref, Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        inputs::validate_inputs,
        video::{VideoConfig, VideoFrame},
        Error, FrameId,
    },
};

pub struct Equalizer {
    input: NodeRef,
}

impl Equalizer {
    pub fn new(inputs: Vec<NodeRef>) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideSpectrum)?;

        Ok(Self { input })
    }
}

impl Node for Equalizer {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideVideoFrame)
    }

    fn provide_video_frame(&self, id: FrameId, frame: &mut VideoFrame) {
        let spectrum = self.input.provide_spectrum(id);
        let n_bins = spectrum.len();
        let bin_width = (frame.width() as f32 / n_bins as f32).ceil() as usize;
        let frame_height = frame.height();

        frame.apply(|(x, y), pixel| {
            let bin = x / bin_width;
            let amplitude = spectrum[bin].norm();
            let bin_height = (amplitude * frame_height as f32).round() as usize;
            // Ignore too-low bins.
            let bin_height = if bin_height < 5 { 0 } else { bin_height };

            if y >= frame_height - bin_height {
                pixel.set_grayscale(255);
            }
        });
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "equalizer"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        _: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        Equalizer::new(inputs).map(node_ref)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
