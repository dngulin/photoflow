use glutin_egl_sys::egl::Egl;
use gstreamer_gl::prelude::GLContextExt;
use gstreamer_gl::{GLContext, GLPlatform};
use gstreamer_gl_egl::GLDisplayEGL;
use slint::GraphicsAPI;

pub trait GLContextSlint {
    fn from_slint_graphics_api(graphics_api: &GraphicsAPI<'_>) -> anyhow::Result<GLContext>;
}

impl GLContextSlint for GLContext {
    fn from_slint_graphics_api(graphics_api: &GraphicsAPI<'_>) -> anyhow::Result<GLContext> {
        let egl = match graphics_api {
            GraphicsAPI::NativeOpenGL { get_proc_address } => {
                Egl::load_with(|symbol| get_proc_address(&std::ffi::CString::new(symbol).unwrap()))
            }
            _ => return Err(anyhow::anyhow!("Unsupported graphics API")),
        };

        let gl_ctx = unsafe { get_gl_ctx_from_egl(&egl)? };

        gl_ctx.activate(true)?;
        gl_ctx.fill_info()?;

        Ok(gl_ctx)
    }
}

unsafe fn get_gl_ctx_from_egl(egl: &Egl) -> anyhow::Result<GLContext> {
    let context = GLContext::new_wrapped(
        &GLDisplayEGL::with_egl_display(egl.GetCurrentDisplay() as usize)?,
        egl.GetCurrentContext() as _,
        GLPlatform::EGL,
        GLContext::current_gl_api(GLPlatform::EGL).0,
    )
    .ok_or_else(|| anyhow::anyhow!("Unable to create wrapped GL context"))?;

    Ok(context)
}
