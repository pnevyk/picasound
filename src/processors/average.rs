use crate::{
    options::Options,
    pipeline::{Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{inputs::validate_inputs, video::VideoConfig, Error, FrameId},
};

pub struct Average {
    input: NodeRef,
    alpha: f32,
    average: f32,
}

impl Average {
    pub fn new(inputs: Vec<NodeRef>, options: Options) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideNumber)?;

        let alpha = options
            .get("smoothing-factor")
            .unwrap_or(&0.5.into())
            .as_f32()
            .ok_or(Error::InvalidOptions)?;

        Ok(Self {
            input,
            alpha,
            average: 0.0,
        })
    }
}

impl Node for Average {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideNumber)
    }

    fn provide_number(&mut self, id: FrameId) -> f32 {
        let current = self.input.provide_number(id);
        self.average = (self.alpha * current) + (1.0 - self.alpha) * self.average;
        self.average
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "average"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        options: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        Average::new(inputs, options).map(NodeRef::new)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
