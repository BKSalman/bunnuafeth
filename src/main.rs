use std::{
    ffi::{c_long, c_uint, CStr, CString},
    slice,
};

use x11_dl::{
    xft::{Xft, XftFont},
    xlib::{self, CapButt, Display, JoinMiter, LineSolid, PropModeReplace, Xlib, XA_WINDOW},
    xrandr::Xrandr,
};

pub const ROOT_EVENT_MASK: c_long = xlib::SubstructureRedirectMask
    | xlib::SubstructureNotifyMask
    | xlib::ButtonPressMask
    | xlib::PointerMotionMask
    | xlib::StructureNotifyMask;

struct Drawable {
    display: *mut Display,
    screen: i32,
    root: xlib::Window,
    width: i32,
    height: i32,
    drawable: x11_dl::xlib::Drawable,
    graphics_context: x11_dl::xlib::GC,
}

#[derive(Default)]
struct Xywh {
    x: i32,
    y: i32,
    h: i32,
    w: i32,
    minw: i32,
    maxw: i32,
    minh: i32,
    maxh: i32,
}

#[derive(Default)]
struct BoundingBox {
    x: i32,
    y: i32,
    height: i32,
    width: i32,
}

impl BoundingBox {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            height,
            width,
        }
    }
}

impl Xywh {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            h: height,
            w: width,
            ..Default::default()
        }
    }
}

enum BarPosition {
    Top,
    Bottom,
}

struct Monitor {
    root: xlib::Window,
    output: String,
    bounding_box: BoundingBox,
    bar: Bar,
}

struct Bar {
    show: bool,
    pos: BarPosition,
    y: i32,
}

impl Monitor {
    fn new() -> Self {
        Self {
            output: String::new(),
            bounding_box: BoundingBox::new(0, 0, 0, 0),
            root: Default::default(),
            bar: Bar {
                show: true,
                pos: BarPosition::Top,
                y: 0,
            },
        }
    }
    fn with_bbox(bounding_box: BoundingBox) -> Self {
        Self {
            output: String::new(),
            bounding_box,
            root: Default::default(),
            bar: Bar {
                show: true,
                pos: BarPosition::Top,
                y: 0,
            },
        }
    }
}

const LEFT_PTR: c_uint = 68;
const SIZING: c_uint = 120;
const FLEUR: c_uint = 52;

fn main() {
    println!("Opening Xlib and Xft");
    let xlib = Xlib::open().expect("connect to the X server");
    let xft = Xft::open().expect("open xft");

    println!("Creating Drawable");
    let drawable = create_drawable(&xlib);

    println!("Loading fonts");

    load_font(&xft, &drawable, "monospace");

    println!("Updating geometry");

    println!("Setting up root window and dummy window");
    unsafe {
        (xlib.XSetLineAttributes)(
            drawable.display,
            drawable.graphics_context,
            1,
            LineSolid,
            CapButt,
            JoinMiter,
        );

        let mut attrs: xlib::XSetWindowAttributes = std::mem::zeroed();
        attrs.cursor = (xlib.XCreateFontCursor)(drawable.display, LEFT_PTR);
        attrs.event_mask = ROOT_EVENT_MASK;

        let wmcheckwin =
            (xlib.XCreateSimpleWindow)(drawable.display, drawable.root, 0, 0, 1, 1, 0, 0, 0);

        let netwmcheck = (xlib.XInternAtom)(
            drawable.display,
            CString::new("_NET_SUPPORTING_WM_CHECK").unwrap().into_raw(),
            xlib::False,
        );

        (xlib.XChangeProperty)(
            drawable.display,
            wmcheckwin,
            netwmcheck,
            XA_WINDOW,
            32,
            PropModeReplace,
            [wmcheckwin as c_long].as_ptr().cast::<u8>(),
            1,
        );

        let utf8string = (xlib.XInternAtom)(
            drawable.display,
            CString::new("UTF8_STRING").unwrap_or_default().into_raw(),
            xlib::False,
        );

        let netwmname = (xlib.XInternAtom)(
            drawable.display,
            CString::new("_NET_WM_NAME").unwrap_or_default().into_raw(),
            xlib::False,
        );

        (xlib.XChangeProperty)(
            drawable.display,
            wmcheckwin,
            netwmname,
            utf8string,
            8,
            PropModeReplace,
            "Bunnuafeth".as_ptr().cast::<u8>(),
            3,
        );

        let netwmcheck = (xlib.XInternAtom)(
            drawable.display,
            CString::new("_NET_SUPPORTING_WM_CHECK")
                .unwrap_or_default()
                .into_raw(),
            xlib::False,
        );

        (xlib.XChangeProperty)(
            drawable.display,
            drawable.root,
            netwmcheck,
            XA_WINDOW,
            32,
            PropModeReplace,
            [wmcheckwin as c_long].as_ptr().cast::<u8>(),
            1,
        );

        let netclientlist = (xlib.XInternAtom)(
            drawable.display,
            CString::new("_NET_CLIENT_LIST")
                .unwrap_or_default()
                .into_raw(),
            xlib::False,
        );

        (xlib.XDeleteProperty)(drawable.display, drawable.root, netclientlist);

        (xlib.XChangeWindowAttributes)(
            drawable.display,
            drawable.root,
            xlib::CWEventMask | xlib::CWCursor,
            &mut attrs,
        );

        (xlib.XSelectInput)(drawable.display, drawable.root, ROOT_EVENT_MASK);

        (xlib.XSync)(drawable.display, xlib::False);

        let mut event: xlib::XEvent = std::mem::zeroed();

        println!("Starting event loop");
        loop {
            if (xlib.XNextEvent)(drawable.display, &mut event) > 0 {
                break;
            }
        }
    }
}

fn create_drawable(xlib: &Xlib) -> Drawable {
    let drawable = unsafe {
        let display = (xlib.XOpenDisplay)(std::ptr::null());

        let screen = (xlib.XDefaultScreen)(display);
        let screen_width = (xlib.XDisplayWidth)(display, screen);
        let screen_height = (xlib.XDisplayHeight)(display, screen);

        let root: xlib::Window = (xlib.XRootWindow)(display, screen);

        let depth = (xlib.XDefaultDepth)(display, screen);

        let drawable = (xlib.XCreatePixmap)(
            display,
            root,
            screen_width as u32,
            screen_height as u32,
            depth as u32,
        );

        let graphics_context = (xlib.XCreateGC)(display, root, 0, std::ptr::null_mut());

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

    drawable
}

fn load_font(xft: &Xft, drawable: &Drawable, fontname: &str) -> *mut XftFont {
    unsafe {
        let xfont = (xft.XftFontOpenName)(
            drawable.display,
            drawable.screen,
            CString::new(fontname).unwrap_or_default().as_ptr(),
        );
        xfont
    }
}

fn get_monitors(drawable: &Drawable) -> Vec<Monitor> {
    let xrandr = Xrandr::open().expect("open xrandr");
    unsafe {
        let screen_resources = (xrandr.XRRGetScreenResources)(drawable.display, drawable.root);
        let outputs = slice::from_raw_parts(
            (*screen_resources).outputs,
            (*screen_resources).noutput as usize,
        );

        return outputs
            .iter()
            .map(|output| (xrandr.XRRGetOutputInfo)(drawable.display, screen_resources, *output))
            .filter(|&output_info| (*output_info).crtc != 0)
            .map(|output_info| {
                let crtc_info = (xrandr.XRRGetCrtcInfo)(
                    drawable.display,
                    screen_resources,
                    (*output_info).crtc,
                );
                let root = *crtc_info;
                let mut monitor = Monitor::with_bbox(BoundingBox::new(
                    root.x,
                    root.y,
                    root.width as i32,
                    root.height as i32,
                ));
                monitor.root = drawable.root;
                monitor.output = CStr::from_ptr((*output_info).name)
                    .to_string_lossy()
                    .into_owned();
                monitor
            })
            .collect();
    }
}

fn update_bar_position(drawable: &Drawable, bar_height: i32) {
    let mut monitors = get_monitors(drawable);

    for monitor in monitors.iter_mut() {
        if monitor.bar.show {
            monitor.bounding_box.height -= bar_height;
            match monitor.bar.pos {
                BarPosition::Top => {
                    monitor.bar.y = monitor.bounding_box.y;
                    monitor.bounding_box.y += bar_height;
                }
                BarPosition::Bottom => {
                    monitor.bar.y = monitor.bounding_box.height + monitor.bounding_box.y;
                }
            }
        } else {
            monitor.bar.y = -bar_height;
        }
    }
}
