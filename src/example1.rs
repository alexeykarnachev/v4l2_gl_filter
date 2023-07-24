use gst::prelude::*;
use anyhow::Error;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {src}: {error} (debug: {debug:?})")]
struct ErrorMessage {
    src: glib::GString,
    error: glib::Error,
    debug: Option<glib::GString>,
}


fn _main() -> Result<(), Error>  {
    // gst-launch-1.0 v4l2src ! video/x-raw,width=640,height=480,framerate=30/1 ! videoconvert ! autovideosink

    gst::init()?;

    let pipeline = gst::Pipeline::default();
    let src = gst::ElementFactory::make("v4l2src").property("device", "/dev/video0").build()?;
    let caps = gst_video::VideoCapsBuilder::new()
        .width(640)
        .height(480)
        .framerate((30, 1).into())
        .build();
    let capsfilter = gst::ElementFactory::make("capsfilter")
        .property("caps", &caps)
        .build()?;
    let videoconvert = gst::ElementFactory::make("videoconvert").build()?;
    let appsink = gst::ElementFactory::make("autovideosink").build()?;

    pipeline.add_many(&[&src, &capsfilter, &videoconvert, &appsink])?;
    gst::Element::link_many(&[&src, &capsfilter, &videoconvert, &appsink])?;

    pipeline.set_state(gst::State::Playing)?;
    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| s.path_string())
                        .unwrap_or_else(|| glib::GString::from("UNKNOWN")),
                    error: err.error(),
                    debug: err.debug(),
                }
                .into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

pub fn main() {
    match run(_main) {
        Ok(r) => r,
        Err(e) => eprintln!("Error! {e}"),
    }
}

pub fn run<T, F: FnOnce() -> T + Send + 'static>(main: F) -> T
where
    T: Send + 'static,
{
    main()
}

