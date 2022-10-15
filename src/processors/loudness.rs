use std::ops::Div;

use crate::{
    options::Options,
    pipeline::{Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{inputs::validate_inputs, video::VideoConfig, Error, FrameId},
};

#[derive(Debug)]
pub struct Loudness {
    input: NodeRef,
}

impl Loudness {
    pub fn new(inputs: Vec<NodeRef>) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideAudioData)?;
        Ok(Self { input })
    }
}

impl Node for Loudness {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideNumber)
    }

    fn provide_number(&mut self, id: FrameId) -> f32 {
        let data = self.input.provide_audio_data(id);
        let data = data.frames(1);
        let rms = data
            .iter()
            .copied()
            .map(|x| x * x)
            .sum::<f32>()
            .div(data.len() as f32)
            .sqrt();
        rms
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "loudness"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        _: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        Loudness::new(inputs).map(NodeRef::new)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
