use std::{env, fs::File};

use picasound::{
    load::PipelineConfig,
    pipeline::{Node, NodeFactory},
};

fn main() {
    let path = env::args()
        .skip(1)
        .last()
        .expect("pipeline file not specified");
    let file = File::open(path).expect("could not read pipeline file");
    let config = PipelineConfig::from_reader(file).unwrap();
    let sinks = config.pipeline(&NodeFactory::default()).unwrap();

    for mut sink in sinks {
        sink.start().unwrap();
    }
}
