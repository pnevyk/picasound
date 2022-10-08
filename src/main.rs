use std::{env, fs::File};

use picasound::{load::PipelineConfig, pipeline::NodeFactory};

fn main() {
    let path = env::args().last().expect("pipeline file not specified");
    let file = File::open(path).expect("could not read pipeline file");
    let config = PipelineConfig::from_reader(file).unwrap();
    let sinks = config.pipeline(&NodeFactory::default()).unwrap();

    for sink in sinks {
        sink.start().unwrap();
    }
}
