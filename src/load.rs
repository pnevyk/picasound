use std::{collections::HashMap, io};

use petgraph::{algo::toposort, Graph};
use serde::Deserialize;

use crate::{
    options::from_yaml,
    pipeline::{NodeFactory, NodeRef, NodeRegistry},
    util::{video::VideoConfig, Error, InvalidPipeline},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PipelineConfig {
    video: Option<PipelineConfigVideo>,
    pipeline: HashMap<String, PipelineConfigNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PipelineConfigVideo {
    width: Option<usize>,
    height: Option<usize>,
    fps: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PipelineConfigNode {
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    inputs: Inputs,
    options: Option<serde_yaml::Mapping>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Inputs {
    One(String),
    Many(Vec<String>),
}

impl Default for Inputs {
    fn default() -> Self {
        Inputs::Many(Vec::new())
    }
}

impl Inputs {
    fn as_slice(&self) -> &[String] {
        match self {
            Inputs::One(one) => std::slice::from_ref(one),
            Inputs::Many(many) => many.as_slice(),
        }
    }
}

impl PipelineConfig {
    pub fn from_reader<R: io::Read>(reader: R) -> serde_yaml::Result<Self> {
        serde_yaml::from_reader(reader)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str<R: io::Read>(string: &str) -> serde_yaml::Result<Self> {
        serde_yaml::from_str(string)
    }

    pub fn video_config(&self) -> VideoConfig {
        match self.video.as_ref() {
            Some(video) => {
                let mut builder = VideoConfig::builder();

                if let Some(width) = video.width {
                    builder.width(width);
                }

                if let Some(height) = video.height {
                    builder.height(height);
                }

                if let Some(fps) = video.fps {
                    builder.fps(fps);
                }

                builder.build()
            }
            None => VideoConfig::default(),
        }
    }

    pub fn pipeline(mut self, factory: &NodeFactory) -> Result<Vec<NodeRef>, Error> {
        let mut sinks = Vec::new();

        for (name, definition) in self.pipeline.iter() {
            let constructor = factory
                .get(&definition.node_type)
                .ok_or_else(|| Error::UnknownNode(definition.node_type.clone()))?;

            if constructor.is_sink() {
                sinks.push(name.clone());
            }
        }

        if sinks.is_empty() {
            return Err(Error::InvalidPipeline(
                InvalidPipeline::SourceProcessorsSinkOrder,
            ));
        }

        let mut graph = Graph::new();
        let mut vertices = HashMap::new();

        for name in self.pipeline.keys() {
            vertices.insert(name.clone(), graph.add_node(name.clone()));
        }

        for (name, definition) in self.pipeline.iter() {
            let v = vertices[name];

            for input in definition.inputs.as_slice() {
                let u = vertices[input];
                graph.add_edge(u, v, ());
            }
        }

        let sorted =
            toposort(&graph, None).map_err(|_| Error::InvalidPipeline(InvalidPipeline::Cycle))?;

        let config = self.video_config();
        let mut registry = NodeRegistry::new();

        for node_id in sorted.into_iter() {
            let node_name = graph[node_id].clone();
            let definition = self.pipeline.remove(&node_name).unwrap();
            let inputs = definition
                .inputs
                .as_slice()
                .iter()
                .map(|input| {
                    registry
                        .get(input)
                        .ok_or(Error::InvalidPipeline(InvalidPipeline::UnknownInput))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let options = from_yaml(definition.options.unwrap_or_default())?;

            let node = factory.construct(&definition.node_type, inputs, options, config)?;

            registry.register(node_name, node);
        }

        let sinks = sinks
            .iter()
            .map(|sink_name| registry.get(sink_name).unwrap())
            .collect();

        Ok(sinks)
    }
}
