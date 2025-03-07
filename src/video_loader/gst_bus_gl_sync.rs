use gstreamer::prelude::*;
use gstreamer::{Bus, BusSyncReply, Context, Element, Message, MessageView};
use gstreamer_gl::prelude::*;
use gstreamer_gl::{GLContext, GL_DISPLAY_CONTEXT_TYPE};

pub trait BusGlSync {
    fn set_gl_sync_handler(&self, ctx: GLContext);
}

impl BusGlSync for Bus {
    fn set_gl_sync_handler(&self, gl_ctx: GLContext) {
        let handler = move |_: &Bus, msg: &Message| {
            if let MessageView::NeedContext(ctx) = msg.view() {
                let ctx_type = ctx.context_type();

                if ctx_type == *GL_DISPLAY_CONTEXT_TYPE {
                    if let Some(element) = msg
                        .src()
                        .and_then(|source| source.downcast_ref::<Element>())
                    {
                        let gst_context = Context::new(ctx_type, true);
                        gst_context.set_gl_display(&gl_ctx.display());
                        element.set_context(&gst_context);
                    }
                } else if ctx_type == "gst.gl.app_context" {
                    if let Some(element) = msg
                        .src()
                        .and_then(|source| source.downcast_ref::<Element>())
                    {
                        let mut gst_context = Context::new(ctx_type, true);
                        {
                            let gst_context = gst_context.get_mut().unwrap();
                            let structure = gst_context.structure_mut();
                            structure.set("context", &gl_ctx);
                        }
                        element.set_context(&gst_context);
                    }
                }
            }

            BusSyncReply::Drop
        };

        self.set_sync_handler(handler);
    }
}
