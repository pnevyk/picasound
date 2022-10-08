use gst_rtsp_server::prelude::*;

use gstreamer_rtsp_server::traits::RTSPServerExt;

use crate::{
    options::Options,
    pipeline::{node_ref, Capability, ConstructNode, Node, NodeFactory, NodeRef},
    util::{
        inputs::validate_inputs,
        video::{VideoConfig, VideoFrame},
        Error, FrameId,
    },
};

const MOUNT_PATH: &str = "/picasound";

pub struct RtspSink {
    main_loop: glib::MainLoop,
    server: gst_rtsp_server::RTSPServer,
    id: Option<glib::SourceId>,
}

impl RtspSink {
    pub fn new(inputs: Vec<NodeRef>, config: VideoConfig) -> Result<Self, Error> {
        let input = validate_inputs(inputs, Capability::ProvideVideoFrame)?;

        gst::init().map_err(|_| Error::System)?;

        let main_loop = glib::MainLoop::new(None, false);
        let server = gst_rtsp_server::RTSPServer::new();
        let mounts = server.mount_points().ok_or(Error::System)?;

        let factory = setup_factory(input, config);

        mounts.add_factory(MOUNT_PATH, &factory);

        let id = Some(server.attach(None).map_err(|_| Error::System)?);

        Ok(Self {
            main_loop,
            server,
            id,
        })
    }

    pub fn start(&self) {
        self.main_loop.run();
    }

    pub fn uri(&self) -> String {
        format!(
            "rtsp://127.0.0.1:{}{}",
            self.server.bound_port(),
            MOUNT_PATH
        )
    }
}

impl Drop for RtspSink {
    fn drop(&mut self) {
        self.main_loop.quit();
        self.id.take().unwrap().remove();
    }
}

// https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/blob/main/examples/src/bin/rtsp-server.rs
// https://github.com/GStreamer/gst-rtsp-server/blob/master/examples/test-appsrc.c
// https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/blob/main/examples/src/bin/appsrc.rs

// play with `gst-launch-1.0 rtspsrc location=rtsp://localhost:8554/test latency=0 ! decodebin ! autovideosink`

fn setup_factory(input: NodeRef, config: VideoConfig) -> gst_rtsp_server::RTSPMediaFactory {
    let factory = gst_rtsp_server::RTSPMediaFactory::new();
    factory
        .set_launch("( appsrc name=source ! videoconvert ! video/x-raw,format=I420 ! x264enc speed-preset=ultrafast tune=zerolatency ! rtph264pay name=pay0 pt=96 )");
    factory.set_shared(true);

    factory.connect_closure(
        "media-configure",
        true,
        glib::closure!(|_: &gst_rtsp_server::RTSPMediaFactory,
                        media: &gst_rtsp_server::RTSPMedia| {
            let input = input.clone();

            let element = media.element().unwrap();
            let source = element
                .dynamic_cast::<gst::Bin>()
                .unwrap()
                .by_name_recurse_up("source")
                .unwrap()
                .dynamic_cast::<gst_app::AppSrc>()
                .unwrap();

            let video_info = gst_video::VideoInfo::builder(
                gst_video::VideoFormat::Bgrx,
                config.width() as u32,
                config.height() as u32,
            )
            .fps(gst::Fraction::new(config.fps() as i32, 1))
            .build()
            .unwrap();

            source.set_format(gst::Format::Time);
            source.set_caps(Some(&video_info.to_caps().unwrap()));

            let mut frame = VideoFrame::new(config.width(), config.height());

            let mut i = 0;
            source.set_callbacks(
                gst_app::AppSrcCallbacks::builder()
                    .need_data(move |source, _| {
                        frame.clear();
                        input.provide_video_frame(FrameId::new(), &mut frame);

                        let mut buffer = gst::Buffer::with_size(video_info.size()).unwrap();
                        {
                            let buffer_ref = buffer.get_mut().unwrap();
                            let clock_time = i * (1000.0 / config.fps() as f32).round() as u64;
                            buffer_ref.set_pts(clock_time * gst::ClockTime::MSECOND);
                            buffer_ref.copy_from_slice(0, frame.buf()).unwrap();
                        };
                        _ = source.push_buffer(buffer);
                        i += 1;
                    })
                    .build(),
            );
        }),
    );

    factory
}

impl Node for RtspSink {
    fn is_sink(&self) -> bool {
        true
    }

    fn start(&self) -> Result<(), Error> {
        self.start();
        Ok(())
    }
}

struct Construct;

impl ConstructNode for Construct {
    fn node_type() -> &'static str
    where
        Self: Sized,
    {
        "rtsp"
    }

    fn construct(
        &self,
        inputs: Vec<NodeRef>,
        _: Options,
        config: VideoConfig,
    ) -> Result<NodeRef, Error> {
        RtspSink::new(inputs, config).map(node_ref)
    }

    fn is_sink(&self) -> bool {
        true
    }
}

pub fn register(factory: &mut NodeFactory) {
    factory.register(Construct);
}
