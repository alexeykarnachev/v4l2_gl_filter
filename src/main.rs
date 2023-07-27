mod renderer;

use renderer::GLFilter;
use std::io;
use std::io::Write;
use std::ops::Deref;
use v4l::io::traits::OutputStream;
use zune_jpeg::JpegDecoder;

use glow::HasContext;
use sdl2::event::Event;
use v4l::buffer::Type::{VideoCapture, VideoOutput};
use v4l::io::traits::CaptureStream;
use v4l::video::{Capture, Output};
use v4l::{Device, FourCC};

use v4l::prelude::*;

fn main() -> io::Result<()> {
    // -------------------------------------------------------------------
    // Initialize source and output video streams
    let src_dev_path = "/dev/video0";
    let out_dev_path = "/dev/video2";

    let src = Device::with_path(src_dev_path)?;
    let mut src_format = Capture::format(&src)?;
    src_format.fourcc = FourCC::new(b"MJPG");
    src_format.width = 640;
    src_format.height = 480;
    src_format = Capture::set_format(&src, &src_format)?;

    let out = Device::with_path(out_dev_path)?;
    let mut out_format = src_format.clone();
    out_format.fourcc = FourCC::new(b"MJPG");
    _ = Output::set_format(&out, &out_format)?;

    let mut src_stream = MmapStream::with_buffers(&src, VideoCapture, 4)?;
    let mut out_stream = MmapStream::with_buffers(&out, VideoOutput, 4)?;

    // -------------------------------------------------------------------
    // Initialize SDL with OpengGL context (and texture, program, etc)
    let sdl2 = sdl2::init().unwrap();
    let timer = sdl2.timer().unwrap();
    let event_pump = Box::leak(Box::new(sdl2.event_pump().unwrap()));
    let mut filter =
        GLFilter::new(&sdl2, src_format.width, src_format.height);

    // -------------------------------------------------------------------
    // Start main loop
    let mut prev_ticks = timer.ticks();

    'main: loop {
        let (src_buf, _) = CaptureStream::next(&mut src_stream)?;
        let (out_buf, out_meta) = OutputStream::next(&mut out_stream)?;
        let out_jpeg = filter.run(src_buf);

        println!(
            "FPS: {:?}",
            1000.0 / (timer.ticks() - prev_ticks) as f32
        );
        prev_ticks = timer.ticks();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                _ => {}
            }
        }

        out_buf[..out_jpeg.len()].clone_from_slice(out_jpeg.deref());
        out_meta.bytesused = out_jpeg.len() as u32;
    }

    Ok(())
}
