use crate::{
    options::Options,
    pipeline::{Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        inputs::validate_inputs,
        video::{VideoConfig, VideoFrame},
        Error, FrameId,
    },
};

pub struct Circle {
    input: NodeRef,
}

impl Circle {
    pub fn new(inputs: Vec<NodeRef>) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideNumber)?;
        Ok(Self { input })
    }
}

impl Node for Circle {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideVideoFrame)
    }

    fn provide_video_frame(&mut self, id: FrameId, frame: &mut VideoFrame) {
        let radius = self.input.provide_number(id);

        let radius = (radius * frame.width().min(frame.height()) as f32) as usize;
        let center = (frame.width() / 2, frame.height() / 2);

        frame.apply(|coords, pixel| {
            let intensity = (in_circle_intensity(coords, center, radius) * 255.0) as u8;
            if intensity > 0 {
                pixel.set_grayscale(intensity)
            }
        });
    }
}

fn in_circle_intensity(coords: (usize, usize), center: (usize, usize), radius: usize) -> f32 {
    let (x, y) = (coords.0 as isize, coords.1 as isize);
    let (cx, cy) = (center.0 as isize, center.1 as isize);

    let r = radius as f32;
    let a = (cx - x).abs() as f32;
    let b = (cy - y).abs() as f32;
    let c = (a.powi(2) + b.powi(2)).sqrt();

    if c <= r {
        // Whole pixel inside the radius.
        1.0
    } else if c <= r + 1.0 {
        // Pixel partially in radius.
        r - c + 1.0
    } else {
        0.0
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "circle"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        _: Options,
        _: VideoConfig,
    ) -> Result<NodeRef, Error> {
        Circle::new(inputs).map(NodeRef::new)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
