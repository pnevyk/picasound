use crate::{
    options::Options,
    pipeline::{Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        video::{VideoConfig, VideoFrame},
        Error, FrameId,
    },
};

#[derive(Debug)]
pub struct Merge {
    inputs: Vec<NodeRef>,
    contributions: Vec<f32>,
    mode: Mode,
}

impl Merge {
    pub fn new(inputs: Vec<NodeRef>, options: Options) -> Result<Self, Error> {
        if inputs
            .iter()
            .any(|input| !input.has_capability(Capability::ProvideVideoFrame))
        {
            return Err(Error::InvalidInputs);
        }

        if inputs.is_empty() {
            return Err(Error::InvalidInputs);
        }

        let contributions = options
            .get("contributions")
            .map(|value| {
                value
                    .as_slice()
                    .ok_or(Error::InvalidOptions)?
                    .iter()
                    .map(|c| c.as_f32().ok_or(Error::InvalidOptions))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_else(|| {
                let n = inputs.len();
                (0..n).map(|_| 1.0 / n as f32).collect::<Vec<_>>()
            });

        let mode = options
            .get("mode")
            .map(|value| match value.as_str() {
                Some("sum") => Ok(Mode::Sum),
                Some("product") => Ok(Mode::Product),
                _ => Err(Error::InvalidOptions),
            })
            .transpose()?
            .unwrap_or(Mode::Sum);

        Ok(Self {
            inputs,
            contributions,
            mode,
        })
    }
}

impl Node for Merge {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideVideoFrame)
    }

    fn provide_video_frame(&mut self, id: FrameId, frame: &mut VideoFrame) {
        self.inputs[0].provide_video_frame(id, frame);

        let c1 = self.contributions[0];

        frame.apply(|_, pixel| {
            pixel.set_red_f(c1 * pixel.red_f());
            pixel.set_green_f(c1 * pixel.green_f());
            pixel.set_blue_f(c1 * pixel.blue_f());
        });

        if self.inputs.len() > 1 {
            let mut frame_copy = frame.clone();
            let mode = self.mode;

            for (input, c) in self
                .inputs
                .iter_mut()
                .zip(self.contributions.iter().copied())
                .skip(1)
            {
                input.provide_video_frame(id, &mut frame_copy);

                frame.apply_zip(&mut frame_copy, |_, pixel1, pixel2| {
                    pixel1.set_red_f(mode.apply(pixel1.red_f(), c * pixel2.red_f()));
                    pixel1.set_green_f(mode.apply(pixel1.green_f(), c * pixel2.green_f()));
                    pixel1.set_blue_f(mode.apply(pixel1.blue_f(), c * pixel2.blue_f()));
                });

                frame_copy.clear();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Sum,
    Product,
}

impl Mode {
    pub fn apply(&self, a: f32, b: f32) -> f32 {
        match self {
            Mode::Sum => a + b,
            Mode::Product => a * b,
        }
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "merge"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        options: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        Merge::new(inputs, options).map(NodeRef::new)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
