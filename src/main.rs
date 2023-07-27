use std::ops::Deref;
use std::{env, io};

use v4l::buffer::Type::{VideoCapture, VideoOutput};
use v4l::io::traits::CaptureStream;
use v4l::io::traits::OutputStream;
use v4l::prelude::*;
use v4l::video::{Capture, Output};
use v4l::{Device, FourCC};

use gl_filter::GLFilter;

mod gl_filter;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        println!(
            "ERROR: Invalid number of arguments.\n\
            Usage: v4l_gl_filter /dev/video<src_id> /dev/video<out_id>",
        );
        std::process::exit(1);
    }

    // -------------------------------------------------------------------
    // Initialize source and output video streams
    let src_dev_path = args[1].clone();
    let out_dev_path = args[2].clone();

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
    // Start main loop
    let mut filter = GLFilter::new(src_format.width, src_format.height);

    loop {
        let (src_buf, _) = CaptureStream::next(&mut src_stream)?;
        let (out_buf, out_meta) = OutputStream::next(&mut out_stream)?;
        let out_jpeg = filter.run(src_buf);

        out_buf[..out_jpeg.len()].clone_from_slice(out_jpeg.deref());
        out_meta.bytesused = out_jpeg.len() as u32;
    }
}
