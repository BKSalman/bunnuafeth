use std::ffi::{c_long, c_uint};

use x11_dl::xlib::{self, CapButt, Display, JoinMiter, LineSolid, Window, Xlib, GC};

pub const ROOT_EVENT_MASK: c_long = xlib::SubstructureRedirectMask
    | xlib::SubstructureNotifyMask
    | xlib::ButtonPressMask
    | xlib::PointerMotionMask
    | xlib::StructureNotifyMask;

struct Drawable {
    display: *mut Display,
    screen: i32,
    root: u64,
    width: i32,
    height: i32,
    drawable: x11_dl::xlib::Drawable,
    graphics_context: x11_dl::xlib::GC,
}

const LEFT_PTR: c_uint = 68;
const SIZING: c_uint = 120;
const FLEUR: c_uint = 52;

fn main() {
    let xlib = Xlib::open().expect("connect to the X server");

    let drawable = unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());

        let screen = (xlib.XDefaultScreen)(display);
        let screen_width = (xlib.XDisplayWidth)(display, screen);
        let screen_height = (xlib.XDisplayHeight)(display, screen);

        let root: Window = (xlib.XRootWindow)(display, screen);

        let depth = (xlib.XDefaultDepth)(display, screen);

        let drawable = (xlib.XCreatePixmap)(
            display,
            root,
            screen_width as u32,
            screen_height as u32,
            depth as u32,
        );

        let graphics_context = (xlib.XCreateGC)(display, root, 0, std::ptr::null_mut());

        (xlib.XSetLineAttributes)(display, graphics_context, 1, LineSolid, CapButt, JoinMiter);

        let mut attrs: xlib::XSetWindowAttributes = std::mem::zeroed();
        attrs.cursor = (xlib.XCreateFontCursor)(display, LEFT_PTR);
        attrs.event_mask = ROOT_EVENT_MASK;

        (xlib.XChangeWindowAttributes)(
            display,
            root,
            xlib::CWEventMask | xlib::CWCursor,
            &mut attrs,
        );

        (xlib.XSelectInput)(display, root, ROOT_EVENT_MASK);

        Drawable {
            display,
            screen,
            root,
            width: screen_width,
            height: screen_height,
            drawable,
            graphics_context,
        }
    };
}
