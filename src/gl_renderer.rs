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
    // Shader uniforms — blur post-processing
    pub blur_type: i32,
    pub blur_strength: f32,
    pub blur_angle: f32,
    // Shader uniforms — bloom/glow post-processing
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub bloom_enabled: bool,
    // Shader uniforms — chromatic aberration post-processing
    pub chromatic_strength: f32,
    pub chromatic_angle: f32,
    pub chromatic_enabled: bool,
    // Post-processing shader programs
    blur_program: Option<ShaderProgram>,
    bloom_program: Option<ShaderProgram>,
    chromatic_program: Option<ShaderProgram>,
    // Ping-pong FBO pair for multi-pass post-processing
    pp_fbos: [glow::Framebuffer; 2],
    pp_textures: [glow::Texture; 2],
    pp_fbo_size: (i32, i32),
    // Current preset name
    pub current_preset: String,
}

/// Helper to create an FBO + texture pair for post-processing
unsafe fn create_pp_fbo_texture(gl: &glow::Context) -> (glow::Framebuffer, glow::Texture) {
    let fbo = gl.create_framebuffer().expect("Failed to create PP FBO");
    let tex = gl.create_texture().expect("Failed to create PP texture");

    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        1,
        1,
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
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.bind_texture(glow::TEXTURE_2D, None);

    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
    gl.framebuffer_texture_2d(
        glow::FRAMEBUFFER,
        glow::COLOR_ATTACHMENT0,
        glow::TEXTURE_2D,
        Some(tex),
        0,
    );
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);

    (fbo, tex)
}

/// Compile a post-processing shader program, returning None on failure
fn compile_pp_shader(gl: &glow::Context, name: &str, source: &str) -> Option<ShaderProgram> {
    let vertex_src = shader_presets::vertex_shader_source();
    match ShaderProgram::new(gl, &vertex_src, source) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("Failed to compile {} shader: {}", name, e);
            None
        }
    }
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

        // Create ping-pong FBO pair
        let (fbo_a, tex_a) = unsafe { create_pp_fbo_texture(&gl) };
        let (fbo_b, tex_b) = unsafe { create_pp_fbo_texture(&gl) };

        // Compile all post-processing shaders
        let blur_program = compile_pp_shader(&gl, "blur", &shader_presets::blur_fragment_source());
        let bloom_program =
            compile_pp_shader(&gl, "bloom", &shader_presets::bloom_fragment_source());
        let chromatic_program = compile_pp_shader(
            &gl,
            "chromatic",
            &shader_presets::chromatic_fragment_source(),
        );

        Self {
            gl,
            program: None,
            vao,
            vbo,
            start_time: std::time::Instant::now(),
            color1: [0.80, 0.33, 0.00],
            color2: [0.93, 0.53, 0.07],
            color3: [1.00, 0.75, 0.15],
            color4: [1.00, 0.92, 0.35],
            angle: std::f32::consts::FRAC_PI_4,
            scale: 1.0,
            speed: 1.0,
            blend: 0.5,
            distort_type: 0,
            distort_strength: 0.0,
            ripple_freq: 2.5,
            noise: 0.0,
            center: 0.0,
            dither: 0.0,
            lighting_type: 0,
            light_strength: 0.0,
            bevel_width: 0.05,
            light_angle: (45.0_f32 - 90.0).to_radians(),
            blur_type: 0,
            blur_strength: 0.5,
            blur_angle: 0.0,
            bloom_threshold: 0.5,
            bloom_intensity: 1.0,
            bloom_enabled: false,
            chromatic_strength: 0.0,
            chromatic_angle: 0.0,
            chromatic_enabled: false,
            blur_program,
            bloom_program,
            chromatic_program,
            pp_fbos: [fbo_a, fbo_b],
            pp_textures: [tex_a, tex_b],
            pp_fbo_size: (1, 1),
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

    /// Ensure both ping-pong FBO textures match the given dimensions.
    fn ensure_pp_fbo_size(&mut self, width: i32, height: i32) {
        if self.pp_fbo_size == (width, height) {
            return;
        }
        unsafe {
            for tex in &self.pp_textures {
                self.gl.bind_texture(glow::TEXTURE_2D, Some(*tex));
                self.gl.tex_image_2d(
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
            }
            self.gl.bind_texture(glow::TEXTURE_2D, None);
        }
        self.pp_fbo_size = (width, height);
    }

    /// Returns true if any post-processing effect is active.
    fn has_active_postprocess(&self) -> bool {
        self.blur_type != 0 || self.bloom_enabled || self.chromatic_enabled
    }

    /// Render the pattern shader (single pass) into the currently bound FBO.
    fn render_scene(&self, width: i32, height: i32) {
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

    /// Run a generic post-processing pass: bind `src_texture` to unit 0,
    /// set `uSceneTexture`, `iResolution`, and call the `setup_uniforms`
    /// closure to set effect-specific uniforms, then draw a fullscreen quad.
    fn run_pp_pass(
        &self,
        program: &ShaderProgram,
        src_texture: glow::Texture,
        width: i32,
        height: i32,
        setup_uniforms: impl FnOnce(&glow::Context, glow::Program),
    ) {
        let gl = &self.gl;
        unsafe {
            gl.viewport(0, 0, width, height);
            gl.use_program(Some(program.id));

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(src_texture));
            if let Some(loc) = gl.get_uniform_location(program.id, "uSceneTexture") {
                gl.uniform_1_i32(Some(&loc), 0);
            }
            if let Some(loc) = gl.get_uniform_location(program.id, "iResolution") {
                gl.uniform_3_f32(Some(&loc), width as f32, height as f32, 1.0);
            }

            setup_uniforms(gl, program.id);

            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.use_program(None);
        }
    }

    /// Render the scene with optional multi-pass post-processing into the
    /// currently bound framebuffer (e.g. GTK's GLArea FBO or export FBO).
    ///
    /// Pipeline order: Scene → Blur → Bloom → Chromatic Aberration
    pub fn render(&mut self, width: i32, height: i32) {
        if !self.has_active_postprocess() {
            // No post-processing: render directly (zero overhead).
            self.render_scene(width, height);
            return;
        }

        // Save the currently bound FBO (GTK's internal FBO or export FBO).
        let saved_fbo = unsafe {
            let mut fbo_id: i32 = 0;
            self.gl
                .get_parameter_i32(glow::FRAMEBUFFER_BINDING)
                .clone_into(&mut fbo_id);
            fbo_id
        };

        self.ensure_pp_fbo_size(width, height);

        // Build the list of active post-processing passes.
        // Each pass is identified so we know which shader/uniforms to use.
        #[derive(Clone, Copy)]
        enum PpPass {
            Blur,
            Bloom,
            Chromatic,
        }

        let mut passes: Vec<PpPass> = Vec::with_capacity(3);
        if self.blur_type != 0 && self.blur_program.is_some() {
            passes.push(PpPass::Blur);
        }
        if self.bloom_enabled && self.bloom_program.is_some() {
            passes.push(PpPass::Bloom);
        }
        if self.chromatic_enabled && self.chromatic_program.is_some() {
            passes.push(PpPass::Chromatic);
        }

        // Render scene into pp_fbos[0] (texture A).
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.pp_fbos[0]));
        }
        self.render_scene(width, height);

        // Ping-pong: source starts as texture A (index 0).
        // Each pass reads from the current source and writes to the other.
        let mut src_idx: usize = 0; // index into pp_textures for the current source

        let num_passes = passes.len();
        for (i, pass) in passes.iter().enumerate() {
            let src_texture = self.pp_textures[src_idx];
            let dst_idx = 1 - src_idx;

            let is_last = i == num_passes - 1;

            // Last pass writes to the saved FBO (final destination).
            if is_last {
                unsafe {
                    if saved_fbo == 0 {
                        self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                    } else {
                        let native = std::num::NonZeroU32::new(saved_fbo as u32).unwrap();
                        self.gl.bind_framebuffer(
                            glow::FRAMEBUFFER,
                            Some(glow::NativeFramebuffer(native)),
                        );
                    }
                }
            } else {
                // Intermediate pass writes to the other ping-pong FBO.
                unsafe {
                    self.gl
                        .bind_framebuffer(glow::FRAMEBUFFER, Some(self.pp_fbos[dst_idx]));
                }
            }

            match pass {
                PpPass::Blur => {
                    let blur_type = self.blur_type;
                    let blur_strength = self.blur_strength;
                    let blur_angle = self.blur_angle;
                    self.run_pp_pass(
                        self.blur_program.as_ref().unwrap(),
                        src_texture,
                        width,
                        height,
                        |gl, prog| unsafe {
                            if let Some(loc) = gl.get_uniform_location(prog, "uBlurType") {
                                gl.uniform_1_i32(Some(&loc), blur_type);
                            }
                            if let Some(loc) = gl.get_uniform_location(prog, "uBlurStrength") {
                                gl.uniform_1_f32(Some(&loc), blur_strength);
                            }
                            if let Some(loc) = gl.get_uniform_location(prog, "uBlurAngle") {
                                gl.uniform_1_f32(Some(&loc), blur_angle);
                            }
                        },
                    );
                }
                PpPass::Bloom => {
                    let threshold = self.bloom_threshold;
                    let intensity = self.bloom_intensity;
                    self.run_pp_pass(
                        self.bloom_program.as_ref().unwrap(),
                        src_texture,
                        width,
                        height,
                        |gl, prog| unsafe {
                            if let Some(loc) = gl.get_uniform_location(prog, "uBloomThreshold") {
                                gl.uniform_1_f32(Some(&loc), threshold);
                            }
                            if let Some(loc) = gl.get_uniform_location(prog, "uBloomIntensity") {
                                gl.uniform_1_f32(Some(&loc), intensity);
                            }
                        },
                    );
                }
                PpPass::Chromatic => {
                    let strength = self.chromatic_strength;
                    let angle = self.chromatic_angle;
                    self.run_pp_pass(
                        self.chromatic_program.as_ref().unwrap(),
                        src_texture,
                        width,
                        height,
                        |gl, prog| unsafe {
                            if let Some(loc) = gl.get_uniform_location(prog, "uChromaticStrength") {
                                gl.uniform_1_f32(Some(&loc), strength);
                            }
                            if let Some(loc) = gl.get_uniform_location(prog, "uChromaticAngle") {
                                gl.uniform_1_f32(Some(&loc), angle);
                            }
                        },
                    );
                }
            }

            // Advance ping-pong index (only matters for intermediate passes).
            if !is_last {
                src_idx = dst_idx;
            }
        }
    }

    /// Render at a specific resolution and return RGBA pixel data
    pub fn render_to_pixels(&mut self, width: i32, height: i32) -> Vec<u8> {
        let (fbo, texture) = unsafe {
            let fbo = self.gl.create_framebuffer().expect("Failed to create FBO");
            let texture = self.gl.create_texture().expect("Failed to create texture");

            self.gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            self.gl.tex_image_2d(
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
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.bind_texture(glow::TEXTURE_2D, None);

            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );

            let status = self.gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                eprintln!("Framebuffer not complete: 0x{:X}", status);
            }
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            (fbo, texture)
        };

        // Bind the export FBO, then render (with post-processing if active) into it.
        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        }
        self.render(width, height);

        unsafe {
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            let mut pixels = vec![0u8; (width * height * 4) as usize];
            self.gl.read_pixels(
                0,
                0,
                width,
                height,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(&mut pixels),
            );

            // Restore default framebuffer
            self.gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.gl.delete_framebuffer(fbo);
            self.gl.delete_texture(texture);

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
            // Clean up main pattern program
            if let Some(program) = self.program.take() {
                self.gl.delete_program(program.id);
            }
            // Clean up post-processing programs
            if let Some(prog) = self.blur_program.take() {
                self.gl.delete_program(prog.id);
            }
            if let Some(prog) = self.bloom_program.take() {
                self.gl.delete_program(prog.id);
            }
            if let Some(prog) = self.chromatic_program.take() {
                self.gl.delete_program(prog.id);
            }
            // Clean up ping-pong FBOs and textures
            for fbo in &self.pp_fbos {
                self.gl.delete_framebuffer(*fbo);
            }
            for tex in &self.pp_textures {
                self.gl.delete_texture(*tex);
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
    gl_area.set_use_es(true);
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

        if let Some(ref mut renderer) = *state_render.borrow_mut() {
            renderer.render(width, height);
        }

        glib::Propagation::Stop
    });

    gl_area
}
