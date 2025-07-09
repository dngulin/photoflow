use crate::winit::{DisplayServer, WinitWindow};
use gstreamer_gl::prelude::GLContextExt;
use gstreamer_gl::{GLContext, GLPlatform};
use slint::{GraphicsAPI, Window};

pub trait GLContextSlint {
    fn from_slint_graphics_api(
        graphics_api: &GraphicsAPI<'_>,
        window: &Window,
    ) -> anyhow::Result<GLContext>;
}

impl GLContextSlint for GLContext {
    fn from_slint_graphics_api(
        graphics_api: &GraphicsAPI<'_>,
        window: &Window,
    ) -> anyhow::Result<GLContext> {
        let gl_ctx = match graphics_api {
            GraphicsAPI::NativeOpenGL { get_proc_address } => {
                get_gst_gl_ctx(get_proc_address, window)?
            }
            _ => anyhow::bail!("Unsupported graphics API"),
        };

        gl_ctx.activate(true)?;
        gl_ctx.fill_info()?;

        Ok(gl_ctx)
    }
}

fn get_gst_gl_ctx(
    get_proc_addr: &dyn Fn(&core::ffi::CStr) -> *const core::ffi::c_void,
    window: &Window,
) -> anyhow::Result<GLContext> {
    match window.display_server()? {
        #[cfg(feature = "wayland")]
        DisplayServer::Wayland => get_gst_gl_ctx_egl(get_proc_addr),

        #[cfg(feature = "x11")]
        DisplayServer::X11 => get_gst_gl_ctx_glx(get_proc_addr),
    }
}

#[cfg(feature = "wayland")]
fn get_gst_gl_ctx_egl(
    get_proc_addr: &dyn Fn(&core::ffi::CStr) -> *const core::ffi::c_void,
) -> anyhow::Result<GLContext> {
    use glutin_egl_sys::egl::Egl;
    use gstreamer_gl_egl::GLDisplayEGL;
    const GL_PLATFORM: GLPlatform = GLPlatform::EGL;

    log::info!("Creating EGL context...");

    let egl = Egl::load_with(|addr| get_proc_addr(&std::ffi::CString::new(addr).unwrap()));

    let context = unsafe {
        GLContext::new_wrapped(
            &GLDisplayEGL::with_egl_display(egl.GetCurrentDisplay() as usize)?,
            egl.GetCurrentContext() as usize,
            GL_PLATFORM,
            GLContext::current_gl_api(GL_PLATFORM).0,
        )
    }
    .ok_or_else(|| anyhow::anyhow!("Unable to create wrapped EGL context"))?;

    Ok(context)
}

#[cfg(feature = "x11")]
fn get_gst_gl_ctx_glx(
    get_proc_addr: &dyn Fn(&core::ffi::CStr) -> *const core::ffi::c_void,
) -> anyhow::Result<GLContext> {
    use glutin_glx_sys::glx::Glx;
    use gstreamer_gl_x11::GLDisplayX11;
    const GL_PLATFORM: GLPlatform = GLPlatform::GLX;

    log::info!("Creating GLX context...");

    let glx = Glx::load_with(|addr| get_proc_addr(&std::ffi::CString::new(addr).unwrap()));

    let context = unsafe {
        GLContext::new_wrapped(
            &GLDisplayX11::with_display(glx.GetCurrentDisplay() as usize)?,
            glx.GetCurrentContext() as usize,
            GL_PLATFORM,
            GLContext::current_gl_api(GL_PLATFORM).0,
        )
    }
    .ok_or_else(|| anyhow::anyhow!("Unable to create wrapped GLX context"))?;

    Ok(context)
}
