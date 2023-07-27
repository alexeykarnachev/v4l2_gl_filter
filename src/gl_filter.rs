use glow::HasContext;
use std::time::{Duration, Instant};
use sdl2::{video::Window, Sdl};
use turbojpeg::OwnedBuf;
use zune_jpeg::JpegDecoder;

pub struct GLFilter {
    time: Instant,

    window: Window,
    gl: glow::Context,
    tex: glow::Texture,
    program: glow::Program,
    width: u32,
    height: u32,

    out_pixels: Vec<u8>,
}

impl GLFilter {
    pub fn new(sdl2: &Sdl, width: u32, height: u32) -> Self {
        let video = sdl2.video().unwrap();
        let window = video
            .window("Limbo", width, height)
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
                width as i32,
                height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                None,
            );

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_BORDER as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_BORDER as i32,
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
                    include_str!("../shaders/screen_rect.vert"),
                ),
                (
                    glow::FRAGMENT_SHADER,
                    include_str!("../shaders/main.frag"),
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

        let out_pixels = vec![0u8; (width * height * 4) as usize];

        Self {
            time: Instant::now(),
            window,
            gl,
            tex,
            program,
            width,
            height,
            out_pixels,
        }
    }

    pub fn run(&mut self, src_jpeg_bytes: &[u8]) -> OwnedBuf {
        let mut decoder = JpegDecoder::new(src_jpeg_bytes);
        let src_pixels = decoder.decode().unwrap();

        let gl = &self.gl;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, self.width as i32, self.height as i32);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                self.width as i32,
                self.height as i32,
                0,
                glow::RGB,
                glow::UNSIGNED_BYTE,
                Some(src_pixels.as_slice()),
            );

            gl.use_program(Some(self.program));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_video_tex").as_ref(),
                0,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_time").as_ref(),
                self.time.elapsed().as_millis() as f32,
            );

            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

            gl.read_pixels(
                0,
                0,
                self.width as i32,
                self.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(self.out_pixels.as_mut()),
            );
        }

        self.window.gl_swap_window();

        let out_jpeg = turbojpeg::compress(
            turbojpeg::Image {
                pixels: self.out_pixels.as_slice(),
                width: self.width as usize,
                height: self.height as usize,
                format: turbojpeg::PixelFormat::RGBA,
                pitch: self.width as usize * 4,
            },
            100,
            turbojpeg::Subsamp::Sub2x2,
        )
        .unwrap();

        out_jpeg
    }
}
