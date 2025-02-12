use crate::*;
use freetype::face::LoadFlag;
use freetype::ffi::FT_LCD_FILTER_DEFAULT;
use freetype::{Error, FtResult, Library, RenderMode};

use freetype_sys::{
    FT_Err_Ok, FT_Library_SetLcdFilter, FT_Load_Char, FT_LOAD_RENDER, FT_RENDER_MODE_SDF,
};
pub use glow::HasContext;

const FONT_SIZE: u32 = 48;

///https://learnopengl.com/img/in-practice/glyph.png
///https://en.wikibooks.org/wiki/OpenGL_Programming/Modern_OpenGL_Tutorial_Text_Rendering_02
#[derive(Debug, Clone, Default)]
pub struct Glyph {
    /// Padding
    pub advance: Vec2,
    pub width: f32,
    pub height: f32,
    pub bearing: Vec2,
    /// X offset of glyph in texture.
    pub uv: f32,
    //TODO: Remove
    pub buffer: Vec<u8>,
}

#[derive(Debug)]
pub struct Atlas {
    pub width: i32,
    pub height: i32,
    pub texture: glow::NativeTexture,
    pub glyphs: [Glyph; 128],
}

impl Atlas {
    //TODO: Figure out how to scale a texture.
    //It does seem like the projection is squishing the font.
    //The big letters like j seem fine but letters like e are squished.
    //I should probably align everything in the texture and save myself the trouble.
    pub fn draw_text(&self, rd: &mut Renderer, text: &str, mut x: f32, mut y: f32, color: Vec4) {
        let start_x = x;
        for c in text.chars() {
            let ch = match self.glyphs.get(c as usize) {
                Some(ch) => ch,
                None => &self.glyphs['?' as usize],
            };

            if c == '\n' {
                y -= self.height as f32;
                x = start_x;
            }

            let xpos = x + ch.bearing.x;
            let ypos = y - (ch.height - ch.bearing.y);

            let w = ch.width;
            let h = ch.height;

            //The projection matrix is top left which flips the y.
            //So we no longer need to flip UV's.
            //~~The y UV is flipped here. !uv.y~~
            let uv_left = ch.uv;
            let uv_right = ch.uv + (ch.width / self.width as f32);
            let uv_top = h / self.height as f32;
            let uv_bottom = 0.0;

            //Top left, Bottom left, Bottom right
            //Bottom right, Top right, Top left
            #[rustfmt::skip]
            let vert = [
                vertex!((xpos, ypos + h),     color, (uv_left, uv_bottom)),
                vertex!((xpos, ypos),         color, (uv_left, uv_top)),
                vertex!((xpos + w, ypos),     color, (uv_right, uv_top)),
                vertex!((xpos + w, ypos),     color, (uv_right, uv_top)),
                vertex!((xpos + w, ypos + h), color, (uv_right, uv_bottom)),
                vertex!((xpos, ypos + h),     color, (uv_left, uv_bottom)),
            ];

            rd.vertices.extend(vert);

            // Advance cursors for the next glyph
            x += (ch.advance.x) as f32;
            //There are no characters with vertical advance without using the `VerticalLayout` flag.
            y += (ch.advance.y) as f32;
        }
    }
}

pub unsafe fn load_font(rd: &Renderer, font: &[u8]) -> Atlas {
    let gl = &rd.gl;

    let lib = Library::init().unwrap();
    // FT_Library_SetLcdFilter(lib.raw(), FT_LCD_FILTER_DEFAULT);

    let mut face = lib.new_memory_face2(font, 0).unwrap();
    face.set_pixel_sizes(0, FONT_SIZE).unwrap();

    let mut width = 0;
    let mut height = 0;
    #[allow(invalid_value)]
    let mut glyphs: [Glyph; 128] = std::mem::zeroed();

    //Load symbols, numbers and letters.
    for i in 32..127 {
        // face.load_char(i, LoadFlag::RENDER).unwrap();
        let err = FT_Load_Char(
            face.raw_mut(),
            i as u32,
            // FT_LOAD_RENDER | FT_RENDER_MODE_SDF as i32,
            FT_LOAD_RENDER,
        );

        if err != FT_Err_Ok {
            panic!("{}", Error::from(err));
        }

        let glyph = face.glyph();
        let bitmap = glyph.bitmap();

        width += bitmap.width();

        if height < bitmap.rows() {
            height = bitmap.rows();
        }

        glyph.render_glyph(RenderMode::Normal).unwrap();

        //Bitshift by 6 to get value in pixels. (2^6 = 64, advance is 1/64 pixels)
        glyphs[i].advance = Vec2::new(
            (glyph.advance().x >> 6) as f32,
            (glyph.advance().y >> 6) as f32,
        );
        glyphs[i].width = bitmap.width() as f32;
        glyphs[i].height = bitmap.rows() as f32;
        glyphs[i].bearing = Vec2::new(glyph.bitmap_left() as f32, glyph.bitmap_top() as f32);
        glyphs[i].buffer = bitmap.buffer().to_vec();
        assert_eq!(
            glyphs[i].buffer.len() as f32,
            glyphs[i].width * glyphs[i].height
        );
    }

    let texture = unsafe { gl.create_texture().unwrap() };
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));

    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::LINEAR as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
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
    gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);

    debug_assert!(width >= 0);
    debug_assert!(width < glow::MAX_TEXTURE_SIZE as i32);
    debug_assert!(height < glow::MAX_TEXTURE_SIZE as i32);

    //If we don't zero this texture, bad things will happen.
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RED as i32,
        width as i32,
        height as i32,
        0,
        glow::RED,
        glow::UNSIGNED_BYTE,
        // Some(&vec![0; (width * height) as usize]),
        None,
    );

    let mut x = 0;

    for i in 32..127 {
        glyphs[i].uv = x as f32 / width as f32;

        if x + glyphs[i].width as i32 > glow::TEXTURE_WIDTH as i32
            || 0 + glyphs[i].height as i32 > glow::TEXTURE_HEIGHT as i32
        {
            panic!("texture is too big!");
        }

        if (glyphs[i].width as i32) < 0 || (glyphs[i].height as i32) < 0 {
            panic!("too small!");
        }

        gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            x,
            0,
            glyphs[i].width as i32,
            glyphs[i].height as i32,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(&glyphs[i].buffer),
        );

        check_error(gl);

        x += glyphs[i].width as i32;
    }

    Atlas {
        width,
        height,
        texture,
        glyphs,
    }
}
