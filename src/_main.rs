use std::io;
use std::sync::{mpsc, RwLock};
use std::thread;

use glow::HasContext;
use sdl2::event::Event;
use v4l::buffer::Type;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::Device;
use v4l::{Format, FourCC};

use v4l::prelude::*;
use v4l::video::capture::Parameters;

fn main() -> io::Result<()> {
    // -------------------------------------------------------------------
    // Initialize video device stream thread
    let buffer_count = 4;

    let mut format: Format;
    let params: Parameters;

    let mut dev =
        RwLock::new(Device::new(0).expect("Failed to open device"));
    {
        let dev = dev.write().unwrap();
        format = dev.format()?;
        params = dev.params()?;

        // try RGB3 first
        format.fourcc = FourCC::new(b"RGB3");
        format = dev.set_format(&format)?;

        if format.fourcc != FourCC::new(b"RGB3") {
            // fallback to Motion-JPEG
            format.fourcc = FourCC::new(b"MJPG");
            format = dev.set_format(&format)?;

            if format.fourcc != FourCC::new(b"MJPG") {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "neither RGB3 nor MJPG supported by the device, but required by this example!",
                ));
            }
        }
    }


    println!("Active format:\n{}", format);
    println!("Active parameters:\n{}", params);

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let dev = dev.write().unwrap();

        // Setup a buffer stream
        let mut stream = MmapStream::with_buffers(
            &dev,
            Type::VideoCapture,
            buffer_count,
        )
        .unwrap();

        loop {
            let (buf, _) = stream.next().unwrap();
            let data = match &format.fourcc.repr {
                b"RGB3" => buf.to_vec(),
                b"MJPG" => {
                    // let mut decoder = jpeg_decoder::Decoder::new(buf);
                    // decoder.decode().expect("failed to decode JPEG")
                    buf.to_vec()
                }
                _ => panic!("invalid buffer pixelformat"),
            };
            tx.send(data).unwrap();
        }
    });

    // -------------------------------------------------------------------
    // Initialize SDL with OpengGL context (and texture, program, etc)
    let sdl2 = sdl2::init().unwrap();
    let timer = sdl2.timer().unwrap();
    let event_pump = Box::leak(Box::new(sdl2.event_pump().unwrap()));
    let video = sdl2.video().unwrap();
    let window = video
        .window("Limbo", 1280, 720)
        .opengl()
        .resizable()
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
            format.width as i32,
            format.height as i32,
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
                    vec3 color = texture(u_tex, vs_texcoord).rgb;
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
    'main: loop {
        let mut dt = (timer.ticks() - prev_ticks) as f32 / 1000.0;
        prev_ticks = timer.ticks();
        println!("{:?}", 1.0 / dt);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                _ => {}
            }
        }

        let data = rx.recv().unwrap();

        /*
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, 640, 480);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGB as i32,
                format.width as i32,
                format.height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(data.as_slice()),
            );

            gl.use_program(Some(program));
            gl.uniform_1_i32(
                gl.get_uniform_location(program, "u_tex").as_ref(),
                0,
            );

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        */

        window.gl_swap_window();
    }

    Ok(())
}
