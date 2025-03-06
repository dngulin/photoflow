use gstreamer_gl::prelude::GLContextExt;
use gstreamer_gl::{GLContext, GLPlatform};
use slint::GraphicsAPI;

#[cfg(feature = "wayland")]
use glutin_egl_sys::egl::Egl as GlImpl;
#[cfg(feature = "wayland")]
use gstreamer_gl_egl::GLDisplayEGL as GlDisplayImpl;
#[cfg(feature = "wayland")]
const GL_PLATFORM: GLPlatform = GLPlatform::EGL;

#[cfg(feature = "x11")]
use glutin_glx_sys::glx::Glx as GlImpl;
#[cfg(feature = "x11")]
use gstreamer_gl_x11::GLDisplayX11 as GlDisplayImpl;
#[cfg(feature = "x11")]
const GL_PLATFORM: GLPlatform = GLPlatform::GLX;

pub trait GLContextSlint {
    fn from_slint_graphics_api(graphics_api: &GraphicsAPI<'_>) -> anyhow::Result<GLContext>;
}

impl GLContextSlint for GLContext {
    fn from_slint_graphics_api(graphics_api: &GraphicsAPI<'_>) -> anyhow::Result<GLContext> {
        let gl = match graphics_api {
            GraphicsAPI::NativeOpenGL { get_proc_address } => GlImpl::load_with(|symbol| {
                get_proc_address(&std::ffi::CString::new(symbol).unwrap())
            }),
            _ => return Err(anyhow::anyhow!("Unsupported graphics API")),
        };

        let gl_ctx = unsafe { get_gst_gl_ctx(&gl)? };

        gl_ctx.activate(true)?;
        gl_ctx.fill_info()?;

        Ok(gl_ctx)
    }
}

unsafe fn get_gst_gl_ctx(gl: &GlImpl) -> anyhow::Result<GLContext> {
    let context = GLContext::new_wrapped(
        &GlDisplayImpl::with_display(gl.GetCurrentDisplay() as usize)?,
        gl.GetCurrentContext() as usize,
        GL_PLATFORM,
        GLContext::current_gl_api(GL_PLATFORM).0,
    )
    .ok_or_else(|| anyhow::anyhow!("Unable to create wrapped GL context"))?;

    Ok(context)
}

#[cfg(feature = "wayland")]
trait WithDisplay {
    unsafe fn with_display(
        display: usize,
    ) -> Result<GlDisplayImpl, gstreamer::glib::error::BoolError>;
}

#[cfg(feature = "wayland")]
impl WithDisplay for GlDisplayImpl {
    unsafe fn with_display(
        display: usize,
    ) -> Result<GlDisplayImpl, gstreamer::glib::error::BoolError> {
        GlDisplayImpl::with_egl_display(display)
    }
}
