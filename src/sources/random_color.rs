use std::sync::{atomic::Ordering, Mutex};

use rand::Rng;

use crate::{
    options::Options,
    pipeline::{node_ref, Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        inputs::validate_inputs,
        misc::FrameCounter,
        video::{VideoConfig, VideoFrame},
        Error, FrameId, LastFrameId,
    },
};

pub struct RandomColor {
    cells: Vec<((usize, usize), (usize, usize))>,
    last_id: LastFrameId,
    frame_counter: FrameCounter,
    cache: Mutex<VideoFrame>,
}

impl RandomColor {
    pub fn new(inputs: Vec<NodeRef>, options: Options, config: VideoConfig) -> Result<Self, Error> {
        validate_inputs(inputs, ())?;

        let split_x = get_borders(&options, "split-x", config.width())?;
        let split_y = get_borders(&options, "split-y", config.height())?;

        let update_every = options
            .get("update-every")
            .unwrap_or(&1000.0.into())
            .as_f32()
            .ok_or(Error::InvalidOptions)?;

        let mut cells = Vec::new();

        for y in split_y.windows(2) {
            for x in split_x.windows(2) {
                cells.push(((x[0], y[0]), (x[1], y[1])));
            }
        }

        let last_id = LastFrameId::default();
        let frame_counter =
            FrameCounter::new((update_every * config.fps() as f32 / 1000.0).round() as usize);
        let cache = Mutex::new(VideoFrame::new(config.width(), config.height()));

        Ok(Self {
            cells,
            last_id,
            frame_counter,
            cache,
        })
    }
}

fn get_borders(options: &Options, name: &str, length: usize) -> Result<Vec<usize>, Error> {
    let mut borders = options
        .get(name)
        .map(|value| {
            value
                .as_slice()
                .ok_or(Error::InvalidOptions)?
                .iter()
                .map(|x| x.as_f32().ok_or(Error::InvalidOptions))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .map(|splits| {
            splits
                .into_iter()
                .map(|x| (x * (length as f32)).round() as usize)
                .collect()
        })
        .unwrap_or_else(Vec::new);

    borders.insert(0, 0);
    borders.push(length);

    Ok(borders)
}

impl Node for RandomColor {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideVideoFrame)
    }

    fn provide_video_frame(&self, id: FrameId, frame: &mut VideoFrame) {
        if self.last_id.store_if_not_eq(id) && self.frame_counter.fetch_inc(Ordering::Relaxed) == 0
        {
            let mut rng = rand::thread_rng();

            if self.cells.len() == 1 {
                let (red, blue, green) = rng.gen();

                frame.apply(|_, pixel| {
                    pixel.set_red(red);
                    pixel.set_green(green);
                    pixel.set_blue(blue);
                });
            } else {
                let cells = &self.cells;
                let colors = cells.iter().map(|_| rng.gen()).collect::<Vec<_>>();

                frame.apply(|(x, y), pixel| {
                    let cell_index = cells
                        .iter()
                        .enumerate()
                        .find(|(_, &((x1, y1), (x2, y2)))| x >= x1 && x < x2 && y >= y1 && y < y2)
                        .unwrap()
                        .0;

                    let (red, green, blue) = colors[cell_index];

                    pixel.set_red(red);
                    pixel.set_green(green);
                    pixel.set_blue(blue);
                });
            }

            self.cache.lock().unwrap().copy_from(frame);
        } else {
            frame.copy_from(&self.cache.lock().unwrap());
        }
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "random-color"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        options: Options,
        config: VideoConfig,
    ) -> Result<NodeRef, Error> {
        RandomColor::new(inputs, options, config).map(node_ref)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
