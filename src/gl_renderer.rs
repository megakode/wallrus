use glow::HasContext;
use gtk4::prelude::*;
use gtk4::{glib, GLArea};
use std::cell::RefCell;
use std::rc::Rc;

use crate::shader::ShaderProgram;
use crate::shader_presets;

/// Load GL function pointers via the platform's native GL proc address loader.
/// On Wayland this uses eglGetProcAddress, on X11 glXGetProcAddress.
/// Falls back to dlsym on the GL library as a last resort.
mod gl_loader {
    use std::ffi::{c_void, CStr};
    use std::sync::OnceLock;

    type GetProcAddr = unsafe extern "C" fn(*const std::ffi::c_char) -> *const c_void;

    static LOADER: OnceLock<GetProcAddr> = OnceLock::new();

    fn find_loader() -> GetProcAddr {
        unsafe {
            // Try EGL first (Wayland and modern systems)
            let egl = libc::dlopen(
                b"libEGL.so.1\0".as_ptr() as *const _,
                libc::RTLD_NOW | libc::RTLD_GLOBAL,
            );
            if !egl.is_null() {
                let sym = libc::dlsym(egl, b"eglGetProcAddress\0".as_ptr() as *const _);
                if !sym.is_null() {
                    return std::mem::transmute(sym);
                }
            }

            // Try GLX (X11)
            let glx = libc::dlopen(
                b"libGLX.so.0\0".as_ptr() as *const _,
                libc::RTLD_NOW | libc::RTLD_GLOBAL,
            );
            if !glx.is_null() {
                let sym = libc::dlsym(glx, b"glXGetProcAddressARB\0".as_ptr() as *const _);
                if !sym.is_null() {
                    return std::mem::transmute(sym);
                }
            }

            panic!("Could not find eglGetProcAddress or glXGetProcAddressARB");
        }
    }

    pub fn get_proc_address(name: &CStr) -> *const c_void {
        let loader = LOADER.get_or_init(find_loader);
        unsafe { loader(name.as_ptr()) }
    }
}

/// Vertex data for a fullscreen quad (two triangles covering NDC -1..1)
const QUAD_VERTICES: [f32; 12] = [
    -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
];

/// Renderer state that persists across frames
pub struct RendererState {
    pub gl: glow::Context,
    pub program: Option<ShaderProgram>,
    pub vao: glow::VertexArray,
    pub vbo: glow::Buffer,
    pub start_time: std::time::Instant,
    // Shader uniforms — palette colors
    pub color1: [f32; 3],
    pub color2: [f32; 3],
    pub color3: [f32; 3],
    pub color4: [f32; 3],
    // Shader uniforms — parameters
    pub angle: f32,
    pub scale: f32,
    pub speed: f32,
    pub blend: f32,
    pub distort_type: i32,
    pub distort_strength: f32,
    pub ripple_freq: f32,
    pub noise: f32,
    pub center: f32,
    pub dither: f32,
    // Shader uniforms — lighting
    pub lighting_type: i32,
    pub light_strength: f32,
    pub bevel_width: f32,
    pub light_angle: f32,
    // Current preset name
    pub current_preset: String,
}

impl RendererState {
    pub fn new(gl: glow::Context) -> Self {
        let (vao, vbo) = unsafe {
            let vao = gl.create_vertex_array().expect("Failed to create VAO");
            let vbo = gl.create_buffer().expect("Failed to create VBO");

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

            let vertex_bytes: &[u8] = std::slice::from_raw_parts(
                QUAD_VERTICES.as_ptr() as *const u8,
                QUAD_VERTICES.len() * std::mem::size_of::<f32>(),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertex_bytes, glow::STATIC_DRAW);

            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            (vao, vbo)
        };

        Self {
            gl,
            program: None,
            vao,
            vbo,
            start_time: std::time::Instant::now(),
            color1: [0.11, 0.25, 0.60],
            color2: [0.90, 0.35, 0.50],
            color3: [0.20, 0.60, 0.40],
            color4: [0.80, 0.70, 0.20],
            angle: std::f32::consts::FRAC_PI_4,
            scale: 1.0,
            speed: 1.0,
            blend: 0.5,
            distort_type: 0,
            distort_strength: 0.0,
            ripple_freq: 15.0,
            noise: 0.0,
            center: 0.0,
            dither: 0.0,
            lighting_type: 0,
            light_strength: 0.0,
            bevel_width: 0.05,
            light_angle: (45.0_f32 - 90.0).to_radians(),
            current_preset: String::from("Bars"),
        }
    }

    /// Load a shader preset by name
    pub fn load_preset(&mut self, name: &str) -> Result<(), String> {
        let vertex_src = shader_presets::vertex_shader_source();
        let fragment_src = shader_presets::fragment_source_for(name)
            .ok_or_else(|| format!("Unknown preset: {}", name))?;

        self.set_shader(&vertex_src, &fragment_src)?;
        self.current_preset = name.to_string();
        Ok(())
    }

    pub fn set_shader(&mut self, vertex_src: &str, fragment_src: &str) -> Result<(), String> {
        if let Some(old_program) = self.program.take() {
            old_program.delete(&self.gl);
        }

        let program = ShaderProgram::new(&self.gl, vertex_src, fragment_src)?;
        self.program = Some(program);
        Ok(())
    }

    pub fn render(&self, width: i32, height: i32) {
        let gl = &self.gl;

        unsafe {
            gl.viewport(0, 0, width, height);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            if let Some(ref program) = self.program {
                gl.use_program(Some(program.id));

                let elapsed = self.start_time.elapsed().as_secs_f32();

                if let Some(loc) = gl.get_uniform_location(program.id, "iResolution") {
                    gl.uniform_3_f32(Some(&loc), width as f32, height as f32, 1.0);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "iTime") {
                    gl.uniform_1_f32(Some(&loc), elapsed);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uColor1") {
                    gl.uniform_3_f32(Some(&loc), self.color1[0], self.color1[1], self.color1[2]);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uColor2") {
                    gl.uniform_3_f32(Some(&loc), self.color2[0], self.color2[1], self.color2[2]);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uColor3") {
                    gl.uniform_3_f32(Some(&loc), self.color3[0], self.color3[1], self.color3[2]);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uColor4") {
                    gl.uniform_3_f32(Some(&loc), self.color4[0], self.color4[1], self.color4[2]);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uAngle") {
                    gl.uniform_1_f32(Some(&loc), self.angle);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uScale") {
                    gl.uniform_1_f32(Some(&loc), self.scale);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uSpeed") {
                    gl.uniform_1_f32(Some(&loc), self.speed);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uBlend") {
                    gl.uniform_1_f32(Some(&loc), self.blend);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uDistortType") {
                    gl.uniform_1_i32(Some(&loc), self.distort_type);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uDistortStrength") {
                    gl.uniform_1_f32(Some(&loc), self.distort_strength);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uRippleFreq") {
                    gl.uniform_1_f32(Some(&loc), self.ripple_freq);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uNoise") {
                    gl.uniform_1_f32(Some(&loc), self.noise);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uCenter") {
                    gl.uniform_1_f32(Some(&loc), self.center);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uDither") {
                    gl.uniform_1_f32(Some(&loc), self.dither);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uLightingType") {
                    gl.uniform_1_i32(Some(&loc), self.lighting_type);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uLightStrength") {
                    gl.uniform_1_f32(Some(&loc), self.light_strength);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uBevelWidth") {
                    gl.uniform_1_f32(Some(&loc), self.bevel_width);
                }
                if let Some(loc) = gl.get_uniform_location(program.id, "uLightAngle") {
                    gl.uniform_1_f32(Some(&loc), self.light_angle);
                }

                gl.bind_vertex_array(Some(self.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
                gl.bind_vertex_array(None);

                gl.use_program(None);
            }
        }
    }

    /// Render at a specific resolution and return RGBA pixel data
    pub fn render_to_pixels(&self, width: i32, height: i32) -> Vec<u8> {
        let gl = &self.gl;

        unsafe {
            let fbo = gl.create_framebuffer().expect("Failed to create FBO");
            let texture = gl.create_texture().expect("Failed to create texture");

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );

            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                eprintln!("Framebuffer not complete: 0x{:X}", status);
            }

            self.render(width, height);

            let mut pixels = vec![0u8; (width * height * 4) as usize];
            gl.read_pixels(
                0,
                0,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );

            // Restore default framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.delete_framebuffer(fbo);
            gl.delete_texture(texture);

            // Flip vertically (OpenGL origin is bottom-left)
            let row_size = (width * 4) as usize;
            let mut flipped = vec![0u8; pixels.len()];
            for y in 0..height as usize {
                let src = &pixels[y * row_size..(y + 1) * row_size];
                let dst = &mut flipped[(height as usize - 1 - y) * row_size..][..row_size];
                dst.copy_from_slice(src);
            }

            flipped
        }
    }
}

impl Drop for RendererState {
    fn drop(&mut self) {
        unsafe {
            // Program is cleaned up via ShaderProgram::delete in set_shader,
            // but handle any remaining program
            if let Some(program) = self.program.take() {
                self.gl.delete_program(program.id);
            }
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_buffer(self.vbo);
        }
    }
}

/// Shared renderer state type used across the application
pub type SharedRendererState = Rc<RefCell<Option<RendererState>>>;

/// Create a new shared renderer state
pub fn new_shared_state() -> SharedRendererState {
    Rc::new(RefCell::new(None))
}

/// Create a GLArea widget connected to the shared renderer state.
/// Also loads the initial shader preset once the GL context is ready.
pub fn create_gl_area(state: SharedRendererState) -> GLArea {
    let gl_area = GLArea::new();
    gl_area.set_auto_render(true);
    gl_area.set_has_depth_buffer(false);
    gl_area.set_has_stencil_buffer(false);
    gl_area.set_vexpand(true);
    gl_area.set_hexpand(true);

    // Realize: create GL context and load initial shader
    let state_realize = state.clone();
    gl_area.connect_realize(move |area| {
        area.make_current();
        if let Some(error) = area.error() {
            eprintln!("GLArea realize error: {}", error);
            return;
        }

        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|name| gl_loader::get_proc_address(name))
        };

        let mut renderer = RendererState::new(gl);
        if let Err(e) = renderer.load_preset("Bars") {
            eprintln!("Failed to load initial shader: {}", e);
        }

        *state_realize.borrow_mut() = Some(renderer);
    });

    // Unrealize: cleanup
    let state_unrealize = state.clone();
    gl_area.connect_unrealize(move |_| {
        *state_unrealize.borrow_mut() = None;
    });

    // Render callback
    let state_render = state.clone();
    gl_area.connect_render(move |area, _ctx| {
        let scale = area.scale_factor();
        let width = area.width() * scale;
        let height = area.height() * scale;

        if let Some(ref renderer) = *state_render.borrow() {
            renderer.render(width, height);
        }

        glib::Propagation::Stop
    });

    gl_area
}
