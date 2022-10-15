use crate::{
    options::Options,
    pipeline::{Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{audio::AudioBuffer, inputs::validate_inputs, video::VideoConfig, Error, FrameId},
};

pub struct DeviceSource {
    stream: streams::StreamHandle,
    buf: AudioBuffer,
}

impl DeviceSource {
    pub fn new(inputs: Vec<NodeRef>, config: VideoConfig) -> Result<Self, Error> {
        validate_inputs(inputs, ())?;

        let sample_rate = streams::get_sample_rate()?;
        let buf = AudioBuffer::new(sample_rate.0 as usize, config.fps());

        let stream = streams::build({
            let buf = buf.clone();
            move |data| {
                buf.push(data);
            }
        })?;

        Ok(Self { stream, buf })
    }

    pub fn play(&self) -> Result<(), Error> {
        streams::play(self.stream.id())
    }

    pub fn pause(&self) -> Result<(), Error> {
        streams::pause(self.stream.id())
    }
}

impl Drop for DeviceSource {
    fn drop(&mut self) {
        _ = streams::pause(self.stream.id());
    }
}

impl Node for DeviceSource {
    fn has_capability(&self, cap: Capability) -> bool {
        matches!(cap, Capability::ProvideAudioData)
    }

    fn provide_audio_data(&mut self, _: FrameId) -> AudioBuffer {
        self.buf.clone()
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "device"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        _: Options,
        config: VideoConfig,
    ) -> Result<NodeRef, Error> {
        DeviceSource::new(inputs, config).map(NodeRef::new)
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}

mod streams {
    // Implementors of `Node` trait must be `Send` and `Sync`, however streams
    // are not `Send` nor `Sync`. For details see
    // https://github.com/RustAudio/cpal/issues/435. In summary, some cards do
    // not support accessing running multiple streams on the same device at the
    // same time. This module implements an interface for communicating with a
    // forever-running thread that manages all streams throughout the
    // application runtime.

    use std::collections::HashMap;

    use cpal::{
        traits::{DeviceTrait, HostTrait, StreamTrait},
        SampleRate, Stream,
    };
    use crossbeam_channel::Sender;
    use once_cell::sync::Lazy;

    use crate::util::Error;

    static MANAGER_THREAD: Lazy<Sender<(Command, Sender<Output>)>> = Lazy::new(|| {
        let (command_sender, command_receiver) =
            crossbeam_channel::unbounded::<(Command, Sender<Output>)>();

        std::thread::spawn(move || {
            let mut streams = HashMap::new();
            let mut counter = 0;

            while let Ok((command, output_sender)) = command_receiver.recv() {
                let output = match command {
                    Command::GetSampleRate => match find_sample_rate() {
                        Some(sample_rate) => Output::SampleRate(sample_rate),
                        None => Output::Error,
                    },
                    Command::Build(callback) => match build_stream(callback) {
                        Some(stream) => {
                            let handle = StreamHandle(counter);
                            counter += 1;

                            streams.insert(handle.id(), stream);
                            Output::Stream(handle)
                        }
                        None => Output::Error,
                    },
                    Command::Play(stream_id) => {
                        match streams
                            .get(&stream_id)
                            .expect("stream has being dropped")
                            .play()
                        {
                            Ok(_) => Output::Success,
                            Err(_) => Output::Error,
                        }
                    }
                    Command::Pause(stream_id) => match streams
                        .get(&stream_id)
                        .expect("stream has being dropped")
                        .pause()
                    {
                        Ok(_) => Output::Success,
                        Err(_) => Output::Error,
                    },
                    Command::Drop(stream_id) => {
                        streams
                            .remove(&stream_id)
                            .expect("stream has being dropped");
                        Output::Success
                    }
                };

                if output_sender.send(output).is_err() {
                    break;
                }
            }
        });

        command_sender
    });

    pub type DataCallback = Box<dyn FnMut(&[f32]) + Send + 'static>;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StreamId(usize);

    pub struct StreamHandle(usize);

    impl StreamHandle {
        pub fn id(&self) -> StreamId {
            StreamId(self.0)
        }
    }

    impl Drop for StreamHandle {
        fn drop(&mut self) {
            _ = send_command(Command::Drop(self.id()));
        }
    }

    enum Command {
        GetSampleRate,
        Build(DataCallback),
        Play(StreamId),
        Pause(StreamId),
        Drop(StreamId),
    }

    enum Output {
        SampleRate(SampleRate),
        Stream(StreamHandle),
        Success,
        Error,
    }

    fn find_sample_rate() -> Option<SampleRate> {
        let host = cpal::default_host();
        let device = host.default_input_device()?;
        let config = device.default_input_config().ok()?;

        Some(config.sample_rate())
    }

    fn build_stream(mut callback: DataCallback) -> Option<Stream> {
        let host = cpal::default_host();
        let device = host.default_input_device()?;
        let config = device.default_input_config().ok()?;

        device
            .build_input_stream(
                &config.config(),
                move |data: &[f32], _| callback(data),
                |_| {},
            )
            .ok()
    }

    fn send_command(command: Command) -> Result<Output, Error> {
        let (sender, receiver) = crossbeam_channel::bounded(1);
        if MANAGER_THREAD.send((command, sender)).is_err() {
            return Err(Error::System);
        }

        receiver.recv().map_err(|_| Error::System)
    }

    pub fn get_sample_rate() -> Result<SampleRate, Error> {
        match send_command(Command::GetSampleRate)? {
            Output::SampleRate(sample_rate) => Ok(sample_rate),
            _ => Err(Error::System),
        }
    }

    pub fn build<F>(callback: F) -> Result<StreamHandle, Error>
    where
        F: FnMut(&[f32]) + Send + 'static,
    {
        match send_command(Command::Build(Box::new(callback)))? {
            Output::Stream(stream) => Ok(stream),
            _ => Err(Error::System),
        }
    }

    pub fn play(stream_id: StreamId) -> Result<(), Error> {
        match send_command(Command::Play(stream_id))? {
            Output::Success => Ok(()),
            _ => Err(Error::System),
        }
    }

    pub fn pause(stream_id: StreamId) -> Result<(), Error> {
        match send_command(Command::Pause(stream_id))? {
            Output::Success => Ok(()),
            _ => Err(Error::System),
        }
    }
}
