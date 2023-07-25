use image;
use std::io;
use std::io::Write;
use std::ops::Deref;
use std::sync::{mpsc, RwLock};
use std::thread;
use std::time::Instant;
use turbojpeg::*;
use v4l::io::traits::OutputStream;

use jpeg_encoder::{ColorType, Encoder};
use zune_jpeg::JpegDecoder;

use glow::HasContext;
use sdl2::event::Event;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::video::{Capture, Output};
use v4l::Device;
use v4l::{Format, FourCC};

use v4l::prelude::*;
use v4l::video::capture::Parameters;

fn main() -> io::Result<()> {
    // -------------------------------------------------------------------
    // Initialize video device stream thread
    let buffer_count = 4;

    let src = Device::with_path("/dev/video0").unwrap();
    let mut src_format = Capture::format(&src)?;
    src_format.width = 640;
    src_format.height = 480;
    Capture::set_format(&src, &src_format)?;

    let mut src_params = Capture::params(&src)?;
    let video_width = src_format.width as i32;
    let video_height = src_format.height as i32;
    println!("src capabilities:\n{}", src.query_caps()?);
    println!("src format:\n{}", src_format);
    println!("src parameters:\n{}", src_params);

    let out = Device::with_path("/dev/video2").unwrap();
    let mut out_format = src_format.clone();
    // out_format.fourcc = FourCC::new(b"BGR4");
    let out_format = Output::set_format(&out, &out_format)?;
    let out_params = Output::params(&out)?;
    println!("out capabilities:\n{}", out.query_caps()?);
    println!("out format:\n{}", out_format);
    println!("out parameters:\n{}", out_params);

    let mut src_stream =
        MmapStream::with_buffers(&src, Type::VideoCapture, buffer_count)?;
    let mut out_stream =
        MmapStream::with_buffers(&out, Type::VideoOutput, buffer_count)?;

    // -------------------------------------------------------------------
    // Initialize SDL with OpengGL context (and texture, program, etc)
    let sdl2 = sdl2::init().unwrap();
    let timer = sdl2.timer().unwrap();
    let event_pump = Box::leak(Box::new(sdl2.event_pump().unwrap()));
    let video = sdl2.video().unwrap();
    let window = video
        .window("Limbo", video_width as u32, video_height as u32)
        .opengl()
        .hidden()
        .build()
        .unwrap();

    let gl_attr = video.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(4, 6);

    let gl_context = window.gl_create_context().unwrap();
    window.gl_make_current(&gl_context).unwrap();
    Box::leak(Box::new(gl_context));

    let gl: glow::Context;
    let tex;
    let program;
    unsafe {
        gl = glow::Context::from_loader_function(|s| {
            video.gl_get_proc_address(s) as *const _
        });
        let vao = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vao));

        tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            video_width,
            video_height,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            None,
        );

        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::REPEAT as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::REPEAT as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );

        program = gl.create_program().expect("Cannot create program");
        let shaders_src = [
            (
                glow::VERTEX_SHADER,
                "
                #version 460 core

                out vec2 vs_texcoord;

                const vec2 RECT_IDX_TO_NDC[4] = vec2[4](
                    vec2(-1.0, -1.0),
                    vec2(1.0, -1.0),
                    vec2(-1.0, 1.0),
                    vec2(1.0, 1.0)
                );

                const vec2 RECT_IDX_TO_UV[4] = vec2[4](
                    vec2(0.0, 0.0),
                    vec2(1.0, 0.0),
                    vec2(0.0, 1.0),
                    vec2(1.0, 1.0)
                );

                void main() {
                    vs_texcoord = RECT_IDX_TO_UV[gl_VertexID];
                    vs_texcoord.y = 1.0 - vs_texcoord.y;
                    gl_Position = vec4(RECT_IDX_TO_NDC[gl_VertexID], 0.0, 1.0);
                }
                ",
            ),
            (
                glow::FRAGMENT_SHADER,
                "
                #version 460 core

                in vec2 vs_texcoord;

                out vec4 frag_color;

                uniform sampler2D u_tex;

                void main() {
                    vec2 uv = vec2(vs_texcoord.x, 1.0 - vs_texcoord.y);
                    vec3 color = texture(u_tex, uv).rgb;
                    color *= vec3(1.0, 0.6, 0.6);
                    frag_color = vec4(color, 1.0);
                }
                ",
            ),
        ];

        let mut shaders = Vec::with_capacity(shaders_src.len());
        for (shader_type, shader_src) in shaders_src.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(shader, shader_src);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                panic!("{}", gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }

        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
    };

    video.gl_set_swap_interval(1).unwrap();

    // -------------------------------------------------------------------
    // Start main loop
    let mut prev_ticks = timer.ticks();
    let mut res_buf = vec![0u8; (video_width * video_height * 4) as usize];

    'main: loop {
        let (src_buf, src_buf_meta) =
            CaptureStream::next(&mut src_stream).unwrap();
        let start = Instant::now();
        let mut decoder = JpegDecoder::new(src_buf);
        let mut pixels = decoder.decode().unwrap();
        println!("DECODE: {:?}", start.elapsed());
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

        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, video_width, video_height);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                video_width,
                video_height,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(pixels.as_slice()),
            );

            gl.use_program(Some(program));
            gl.uniform_1_i32(
                gl.get_uniform_location(program, "u_tex").as_ref(),
                0,
            );

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            gl.read_pixels(
                0,
                0,
                video_width,
                video_height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(res_buf.as_mut_slice()),
            );
        }

        window.gl_swap_window();

        let (out_buf, out_buf_meta) = OutputStream::next(&mut out_stream)?;
        let start = Instant::now();

        let img = turbojpeg::Image {
            pixels: res_buf.as_slice(),
            width: video_width as usize,
            height: video_height as usize,
            format: turbojpeg::PixelFormat::RGBA,
            pitch: video_width as usize * 4,
        };
        let jpeg_data =
            turbojpeg::compress(img, 100, turbojpeg::Subsamp::Sub2x2)
                .unwrap();
        let jpeg_data = jpeg_data.deref();
        out_buf[..jpeg_data.len()].clone_from_slice(jpeg_data);
        println!("ENCODE: {:?}", start.elapsed());
        out_buf_meta.bytesused = out_buf.len() as u32;
        out_buf_meta.field = 0;
    }

    Ok(())
}
