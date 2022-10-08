use std::{borrow::Borrow, collections::HashMap, hash::Hash, sync::Arc};

use crate::{
    options::Options,
    processors::{average, circle, equalizer, loudness, merge, spectrum},
    sinks::rtsp,
    sources::{device, random_color},
    util::{
        audio::AudioBuffer,
        spectrum::Spectrum,
        video::{VideoConfig, VideoFrame},
        Error, FrameId,
    },
};

#[allow(unused_variables)]
pub trait Node: Send + Sync {
    fn is_sink(&self) -> bool {
        false
    }

    fn start(&self) -> Result<(), Error> {
        assert!(self.is_sink(), "only sinks can be started");
        Ok(())
    }

    fn has_capability(&self, cap: Capability) -> bool {
        false
    }

    fn provide_audio_data(&self, id: FrameId) -> &AudioBuffer {
        panic!("provide_audio_data not available")
    }

    fn provide_video_frame(&self, id: FrameId, frame: &mut VideoFrame) {
        panic!("provide_video_frame not available")
    }

    fn provide_spectrum(&self, id: FrameId) -> Spectrum {
        panic!("provide_spectrum not available")
    }

    fn provide_number(&self, id: FrameId) -> f32 {
        panic!("provide_number not available")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Capability {
    ProvideAudioData,
    ProvideVideoFrame,
    ProvideSpectrum,
    ProvideNumber,
}

pub type NodeRef = Arc<dyn Node>;

pub fn node_ref<N: Node + 'static>(node: N) -> NodeRef {
    Arc::new(node)
}

pub struct NodeRegistry {
    registry: HashMap<String, NodeRef>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn register<K>(&mut self, name: K, node: NodeRef)
    where
        K: Into<String>,
    {
        self.registry.insert(name.into(), node);
    }

    pub fn get<K>(&self, name: &K) -> Option<NodeRef>
    where
        String: Borrow<K>,
        K: Hash + Eq,
    {
        self.registry.get(name).cloned()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ConstructNode {
    fn node_type() -> &'static str
    where
        Self: Sized;

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        options: Options,
        config: VideoConfig,
    ) -> Result<NodeRef, Error>;

    fn is_sink(&self) -> bool {
        false
    }
}

pub struct NodeFactory {
    constructors: HashMap<&'static str, Box<dyn ConstructNode>>,
}

impl NodeFactory {
    pub fn empty() -> Self {
        Self {
            constructors: HashMap::new(),
        }
    }

    pub fn register<N>(&mut self, constructor: N) -> &mut Self
    where
        N: ConstructNode + 'static,
    {
        let existing = self
            .constructors
            .insert(N::node_type(), Box::new(constructor))
            .is_some();

        assert!(!existing, "node constructor with duplicate type registered");

        self
    }

    pub fn construct(
        &self,
        node_type: &str,
        inputs: Vec<NodeRef>,
        options: Options,
        config: VideoConfig,
    ) -> Result<NodeRef, Error> {
        self.constructors
            .get(node_type)
            .ok_or_else(|| Error::UnknownNode(node_type.to_string()))?
            .construct(inputs, options, config)
    }

    pub fn get(&self, node_type: &str) -> Option<&dyn ConstructNode> {
        self.constructors
            .get(node_type)
            .map(|constructor| &**constructor)
    }
}

impl Default for NodeFactory {
    fn default() -> Self {
        let mut factory = Self::empty();

        device::register(&mut factory);
        random_color::register(&mut factory);

        rtsp::register(&mut factory);

        average::register(&mut factory);
        circle::register(&mut factory);
        equalizer::register(&mut factory);
        loudness::register(&mut factory);
        merge::register(&mut factory);
        spectrum::register(&mut factory);

        factory
    }
}