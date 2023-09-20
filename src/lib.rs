use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    ffi::{c_char, c_int, c_long, c_uint, CString},
    marker::PhantomData,
};
use x11_dl::{
    xft::XftFont,
    xlib::{
        self, ButtonPressMask, ExposureMask, ParentRelative, Window, XSetWindowAttributes, Xlib,
    },
};
use x11rb::{
    connection::Connection,
    cursor::Handle as CursorHandle,
    protocol::{
        randr::{ConnectionExt as XrandrConnectionExt, NotifyMask},
        xproto::{
            AtomEnum, ChangeWindowAttributesAux, ConnectionExt as XlibConnectionExt, CoordMode,
            CreateGCAux, CreateWindowAux, EventMask, Gcontext, GetGeometryReply, MapState, Point,
            PropMode, SetMode, WindowClass,
        },
        ErrorKind,
    },
    rust_connection::{ReplyError, RustConnection},
    wrapper::ConnectionExt as WrapperConnectionExt,
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, CURRENT_TIME,
};

pub const ROOT_EVENT_MASK: c_long = xlib::SubstructureRedirectMask
    | xlib::SubstructureNotifyMask
    | xlib::ButtonPressMask
    | xlib::PointerMotionMask
    | xlib::StructureNotifyMask;
pub const LEFT_PTR: c_uint = 68;
pub const SIZING: c_uint = 120;
pub const FLEUR: c_uint = 52;
pub const BAR_HEIGHT: u16 = 30;

pub trait BunnuConnectionExt {
    fn bunnu_create_simple_window(
        &self,
        win_id: u32,
        parent: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        border_width: u16,
        value_list: &CreateWindowAux,
    ) -> Result<(), XlibError>;
}

impl<C> BunnuConnectionExt for C
where
    C: Connection,
{
    fn bunnu_create_simple_window(
        &self,
        win_id: u32,
        parent: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        border_width: u16,
        value_list: &CreateWindowAux,
    ) -> Result<(), XlibError> {
        // (xlib.XCreateSimpleWindow)(drawable.display, drawable.root, 0, 0, 1, 1, 0, 0, 0);
        self.create_window(
            COPY_DEPTH_FROM_PARENT,
            win_id,
            parent,
            x,
            y,
            width,
            height,
            border_width,
            WindowClass::INPUT_OUTPUT,
            COPY_FROM_PARENT,
            value_list,
        )?;

        Ok(())
    }
}

pub struct WM<'a, C: Connection> {
    pub connection: &'a C,
    pub xlib: x11_dl::xlib::Xlib,
    pub display: *mut x11_dl::xlib::Display,
    pub cursors: Cursors,
    pub fonts: Vec<Font>,
    pub screen_num: usize,
    pub windows: Vec<WindowState>,
    pub black_gc: Gcontext,
    pub sequences_to_ignore: BinaryHeap<Reverse<u16>>,
}

pub struct WindowState {
    x: i16,
    y: i16,
    width: u16,
    window: x11rb::protocol::xproto::Window,
    frame_window: x11rb::protocol::xproto::Window,
}

impl WindowState {
    fn new(
        window: x11rb::protocol::xproto::Window,
        frame_window: x11rb::protocol::xproto::Window,
        geom: &GetGeometryReply,
    ) -> WindowState {
        WindowState {
            window,
            frame_window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
        }
    }

    fn close_x_position(&self) -> i16 {
        std::cmp::max(0, self.width - BAR_HEIGHT) as _
    }
}

impl<'a, C: Connection> WM<'a, C> {
    // pub fn load_font(&mut self, xft: &Xft, fontname: &str) -> *mut XftFont {
    //     unsafe {
    //         let xfont = (xft.XftFontOpenName)(
    //             self.display,
    //             self.screen,
    //             CString::new(fontname).unwrap_or_default().as_ptr(),
    //         );
    //         self.fonts.push(Font {
    //             xfont,
    //             height: (*xfont).ascent + (*xfont).descent,
    //         });
    //         xfont
    //     }
    // }

    pub fn draw_bar(&self, state: WindowState) -> Result<(), XlibError> {
        let close_x = state.close_x_position();
        self.connection.poly_line(
            CoordMode::ORIGIN,
            state.frame_window,
            self.black_gc,
            &[
                Point { x: close_x, y: 0 },
                Point {
                    x: state.width as _,
                    y: BAR_HEIGHT as _,
                },
            ],
        )?;
        self.connection.poly_line(
            CoordMode::ORIGIN,
            state.frame_window,
            self.black_gc,
            &[
                Point {
                    x: close_x,
                    y: BAR_HEIGHT as _,
                },
                Point {
                    x: state.width as _,
                    y: 0,
                },
            ],
        )?;
        let reply = self
            .connection
            .get_property(
                false,
                state.window,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                0,
                std::u32::MAX,
            )?
            .reply()?;
        self.connection
            .image_text8(state.frame_window, self.black_gc, 1, 10, &reply.value)?;

        Ok(())
    }

    // pub fn text_width(
    //     &self,
    //     xft: &Xft,
    //     max_width: u32,
    //     left_pad: u32,
    //     text: &str,
    // ) -> Result<u32, XlibError> {
    //     // if (!drw || (render && (!drw->scheme || !w)) || !text || !drw->fonts)
    //     if self.fonts.len() < 1 {
    //         return Err(XlibError::NoFontLoaded);
    //     }

    //     let mut x = 0;
    //     let mut ellipsis_width = 0;
    //     let mut ellipsis_x = 0;
    //     let mut ellipsis_len = 0;
    //     let available_width = 0;
    //     let extent_x = 0;

    //     for c in text.chars() {
    //         for font in self.fonts.iter() {
    //             unsafe {
    //                 if (xft.XftCharExists)(self.display, font.xfont, c.into()) == xlib::True {
    //                     let text_len = text.len();

    //                     let (text_width, height) = {
    //                         let mut glyph_info: XGlyphInfo = std::mem::zeroed();
    //                         (xft.XftTextExtentsUtf8)(
    //                             self.display,
    //                             font.xfont,
    //                             text.as_ptr(),
    //                             c.len_utf8().try_into().unwrap(),
    //                             &mut glyph_info,
    //                         );

    //                         (glyph_info.width, font.height)
    //                     };

    //                     if available_width + ellipsis_width <= text_width {
    //                         ellipsis_x = x + available_width;
    //                         ellipsis_width = text_width - available_width;
    //                         ellipsis_len = text_len;
    //                     }

    //                     if (available_width + text_width) as u32 > max_width {
    //                         x += text_width;
    //                     } else {
    //                     }

    //                     break;
    //                 }
    //             }
    //         }
    //     }

    //     Ok(0)
    // }

    fn find_window_by_id(&self, win: x11rb::protocol::xproto::Window) -> Option<&WindowState> {
        self.windows
            .iter()
            .find(|state| state.window == win || state.frame_window == win)
    }

    /// Scan for already existing windows and manage them
    fn scan_windows(&mut self) -> Result<(), XlibError> {
        // Get the already existing top-level windows.
        let screen = &self.connection.setup().roots[self.screen_num];
        let tree_reply = self.connection.query_tree(screen.root)?.reply()?;

        // For each window, request its attributes and geometry *now*
        let mut cookies = Vec::with_capacity(tree_reply.children.len());
        for win in tree_reply.children {
            let attr = self.connection.get_window_attributes(win)?;
            let geom = self.connection.get_geometry(win)?;
            cookies.push((win, attr, geom));
        }
        // Get the replies and manage windows
        for (win, attr, geom) in cookies {
            if let (Ok(attr), Ok(geom)) = (attr.reply(), geom.reply()) {
                if !attr.override_redirect && attr.map_state != MapState::UNMAPPED {
                    self.manage_window(win, &geom)?;
                }
            } else {
                // Just skip this window
            }
        }

        Ok(())
    }

    fn manage_window(
        &mut self,
        win: x11rb::protocol::xproto::Window,
        geom: &GetGeometryReply,
    ) -> Result<(), XlibError> {
        println!("Managing window {:?}", win);
        let screen = &self.connection.setup().roots[self.screen_num];
        assert!(self.find_window_by_id(win).is_none());

        let frame_win = self.connection.generate_id()?;
        let win_aux = CreateWindowAux::new()
            .event_mask(
                EventMask::EXPOSURE
                    | EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::POINTER_MOTION
                    | EventMask::ENTER_WINDOW,
            )
            .background_pixel(screen.white_pixel);
        self.connection.create_window(
            COPY_DEPTH_FROM_PARENT,
            frame_win,
            screen.root,
            geom.x,
            geom.y,
            geom.width,
            geom.height + BAR_HEIGHT,
            1,
            WindowClass::INPUT_OUTPUT,
            0,
            &win_aux,
        )?;

        self.connection.grab_server()?;
        self.connection.change_save_set(SetMode::INSERT, win)?;
        let cookie = self
            .connection
            .reparent_window(win, frame_win, 0, BAR_HEIGHT as _)?;
        self.connection.map_window(win)?;
        self.connection.map_window(frame_win)?;
        self.connection.ungrab_server()?;

        self.windows.push(WindowState::new(win, frame_win, geom));

        // Ignore all events caused by reparent_window(). All those events have the sequence number
        // of the reparent_window() request, thus remember its sequence number. The
        // grab_server()/ungrab_server() is done so that the server does not handle other clients
        // in-between, which could cause other events to get the same sequence number.
        self.sequences_to_ignore
            .push(Reverse(cookie.sequence_number() as u16));
        Ok(())
    }
}

pub struct Font {
    xfont: *mut XftFont,
    height: i32,
}

pub struct Cursors {
    pub normal: xlib::Cursor,
    pub resize: xlib::Cursor,
    pub r#move: xlib::Cursor,
}

x11rb::atom_manager! {
    pub Atoms : AtomsCookie {
        _NET_ACTIVE_WINDOW,
        _NET_SUPPORTED,
        _NET_SUPPORTING_WM_CHECK,
        _NET_WM_ALLOWED_ACTIONS,
        _NET_WM_PID,

        _NET_WM_STATE,
        _NET_WM_STATE_MODAL,
        _NET_WM_STATE_STICKY,
        _NET_WM_STATE_MAXIMIZED_VERT,
        _NET_WM_STATE_MAXIMIZED_HORZ,
        _NET_WM_STATE_SHADED,
        _NET_WM_STATE_SKIP_TASKBAR,
        _NET_WM_STATE_SKIP_PAGER,
        _NET_WM_STATE_HIDDEN,
        _NET_WM_STATE_FULLSCREEN,
        _NET_WM_STATE_ABOVE,
        _NET_WM_STATE_BELOW,
        _NET_WM_STATE_DEMANDS_ATTENTION,

        _NET_WM_ACTION_MOVE,
        _NET_WM_ACTION_RESIZE,
        _NET_WM_ACTION_MINIMIZE,
        _NET_WM_ACTION_SHADE,
        _NET_WM_ACTION_STICK,
        _NET_WM_ACTION_MAXIMIZE_HORZ,
        _NET_WM_ACTION_MAXIMIZE_VERT,
        _NET_WM_ACTION_FULLSCREEN,
        _NET_WM_ACTION_CHANGE_DESKTOP,
        _NET_WM_ACTION_CLOSE,

        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_DESKTOP,
        _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_WINDOW_TYPE_TOOLBAR,
        _NET_WM_WINDOW_TYPE_MENU,
        _NET_WM_WINDOW_TYPE_UTILITY,
        _NET_WM_WINDOW_TYPE_SPLASH,
        _NET_WM_WINDOW_TYPE_DIALOG,

        _NET_CLIENT_LIST,
        _NET_DESKTOP_VIEWPORT,
        _NET_NUMBER_OF_DESKTOPS,
        _NET_CURRENT_DESKTOP,
        _NET_DESKTOP_NAMES,
        _NET_WM_DESKTOP,
        _NET_WM_STRUT,

        _NET_WM_NAME,
    }
}

type Atom = u32;

impl Atoms {
    fn net_supported(&self) -> Vec<Atom> {
        vec![
            self._NET_ACTIVE_WINDOW,
            self._NET_SUPPORTED,
            self._NET_WM_NAME,
            self._NET_WM_STATE,
            self._NET_WM_ALLOWED_ACTIONS,
            self._NET_WM_PID,
            self._NET_WM_STATE_MODAL,
            self._NET_WM_STATE_STICKY,
            self._NET_WM_STATE_MAXIMIZED_VERT,
            self._NET_WM_STATE_MAXIMIZED_HORZ,
            self._NET_WM_STATE_SHADED,
            self._NET_WM_STATE_SKIP_TASKBAR,
            self._NET_WM_STATE_SKIP_PAGER,
            self._NET_WM_STATE_HIDDEN,
            self._NET_WM_STATE_FULLSCREEN,
            self._NET_WM_STATE_ABOVE,
            self._NET_WM_STATE_BELOW,
            self._NET_WM_STATE_DEMANDS_ATTENTION,
            self._NET_WM_ACTION_MOVE,
            self._NET_WM_ACTION_RESIZE,
            self._NET_WM_ACTION_MINIMIZE,
            self._NET_WM_ACTION_SHADE,
            self._NET_WM_ACTION_STICK,
            self._NET_WM_ACTION_MAXIMIZE_HORZ,
            self._NET_WM_ACTION_MAXIMIZE_VERT,
            self._NET_WM_ACTION_FULLSCREEN,
            self._NET_WM_ACTION_CHANGE_DESKTOP,
            self._NET_WM_ACTION_CLOSE,
            self._NET_WM_WINDOW_TYPE,
            self._NET_WM_WINDOW_TYPE_DESKTOP,
            self._NET_WM_WINDOW_TYPE_DOCK,
            self._NET_WM_WINDOW_TYPE_TOOLBAR,
            self._NET_WM_WINDOW_TYPE_MENU,
            self._NET_WM_WINDOW_TYPE_UTILITY,
            self._NET_WM_WINDOW_TYPE_SPLASH,
            self._NET_WM_WINDOW_TYPE_DIALOG,
            self._NET_SUPPORTING_WM_CHECK,
            self._NET_CLIENT_LIST,
            self._NET_DESKTOP_VIEWPORT,
            self._NET_NUMBER_OF_DESKTOPS,
            self._NET_CURRENT_DESKTOP,
            self._NET_DESKTOP_NAMES,
            self._NET_WM_DESKTOP,
            self._NET_WM_DESKTOP,
            self._NET_WM_STRUT,
        ]
    }
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct XAtoms {
    pub WMProtocols: xlib::Atom,
    pub WMDelete: xlib::Atom,
    pub WMState: xlib::Atom,
    pub WMClass: xlib::Atom,
    pub WMTakeFocus: xlib::Atom,
    pub NetActiveWindow: xlib::Atom,
    pub NetSupported: xlib::Atom,
    pub NetWMName: xlib::Atom,
    pub NetWMState: xlib::Atom,
    pub NetWMAction: xlib::Atom,
    pub NetWMPid: xlib::Atom,

    pub NetWMActionMove: xlib::Atom,
    pub NetWMActionResize: xlib::Atom,
    pub NetWMActionMinimize: xlib::Atom,
    pub NetWMActionShade: xlib::Atom,
    pub NetWMActionStick: xlib::Atom,
    pub NetWMActionMaximizeHorz: xlib::Atom,
    pub NetWMActionMaximizeVert: xlib::Atom,
    pub NetWMActionFullscreen: xlib::Atom,
    pub NetWMActionChangeDesktop: xlib::Atom,
    pub NetWMActionClose: xlib::Atom,

    pub NetWMStateModal: xlib::Atom,
    pub NetWMStateSticky: xlib::Atom,
    pub NetWMStateMaximizedVert: xlib::Atom,
    pub NetWMStateMaximizedHorz: xlib::Atom,
    pub NetWMStateShaded: xlib::Atom,
    pub NetWMStateSkipTaskbar: xlib::Atom,
    pub NetWMStateSkipPager: xlib::Atom,
    pub NetWMStateHidden: xlib::Atom,
    pub NetWMStateFullscreen: xlib::Atom,
    pub NetWMStateAbove: xlib::Atom,
    pub NetWMStateBelow: xlib::Atom,
    pub NetWMStateDemandsAttention: xlib::Atom,

    pub NetWMWindowType: xlib::Atom,
    pub NetWMWindowTypeDesktop: xlib::Atom,
    pub NetWMWindowTypeDock: xlib::Atom,
    pub NetWMWindowTypeToolbar: xlib::Atom,
    pub NetWMWindowTypeMenu: xlib::Atom,
    pub NetWMWindowTypeUtility: xlib::Atom,
    pub NetWMWindowTypeSplash: xlib::Atom,
    pub NetWMWindowTypeDialog: xlib::Atom,

    pub NetSupportingWmCheck: xlib::Atom,
    pub NetClientList: xlib::Atom,
    pub NetDesktopViewport: xlib::Atom,
    pub NetNumberOfDesktops: xlib::Atom,
    pub NetCurrentDesktop: xlib::Atom,
    pub NetDesktopNames: xlib::Atom,
    pub NetWMDesktop: xlib::Atom,
    pub NetWMStrutPartial: xlib::Atom, // net version - Reserve Screen Space
    pub NetWMStrut: xlib::Atom,        // old version

    pub UTF8String: xlib::Atom,
}

impl XAtoms {
    fn new(connection: &RustConnection) -> Self {
        Self::from(connection)
    }

    fn net_supported(&self) -> Vec<xlib::Atom> {
        vec![
            self.NetActiveWindow,
            self.NetSupported,
            self.NetWMName,
            self.NetWMState,
            self.NetWMAction,
            self.NetWMPid,
            self.NetWMStateModal,
            self.NetWMStateSticky,
            self.NetWMStateMaximizedVert,
            self.NetWMStateMaximizedHorz,
            self.NetWMStateShaded,
            self.NetWMStateSkipTaskbar,
            self.NetWMStateSkipPager,
            self.NetWMStateHidden,
            self.NetWMStateFullscreen,
            self.NetWMStateAbove,
            self.NetWMStateBelow,
            self.NetWMStateDemandsAttention,
            self.NetWMActionMove,
            self.NetWMActionResize,
            self.NetWMActionMinimize,
            self.NetWMActionShade,
            self.NetWMActionStick,
            self.NetWMActionMaximizeHorz,
            self.NetWMActionMaximizeVert,
            self.NetWMActionFullscreen,
            self.NetWMActionChangeDesktop,
            self.NetWMActionClose,
            self.NetWMWindowType,
            self.NetWMWindowTypeDesktop,
            self.NetWMWindowTypeDock,
            self.NetWMWindowTypeToolbar,
            self.NetWMWindowTypeMenu,
            self.NetWMWindowTypeUtility,
            self.NetWMWindowTypeSplash,
            self.NetWMWindowTypeDialog,
            self.NetSupportingWmCheck,
            self.NetClientList,
            self.NetDesktopViewport,
            self.NetNumberOfDesktops,
            self.NetCurrentDesktop,
            self.NetDesktopNames,
            self.NetWMDesktop,
            self.NetWMStrutPartial,
            self.NetWMStrut,
        ]
    }
}

impl From<&RustConnection> for XAtoms {
    fn from(connection: &RustConnection) -> Self {
        let get_atom = |atom: &str| {
            connection
                .intern_atom(false, atom.as_bytes())
                .expect("get atom")
                .reply()
                .unwrap()
                .atom
                .into()
        };

        Self {
            WMProtocols: get_atom("WM_PROTOCOLS"),
            WMDelete: get_atom("WM_DELETE_WINDOW"),
            WMState: get_atom("WM_STATE"),
            WMClass: get_atom("WM_CLASS"),
            WMTakeFocus: get_atom("WM_TAKE_FOCUS"),
            NetActiveWindow: get_atom("_NET_ACTIVE_WINDOW"),
            NetSupported: get_atom("_NET_SUPPORTED"),
            NetWMName: get_atom("_NET_WM_NAME"),
            NetWMPid: get_atom("_NET_WM_PID"),

            NetWMState: get_atom("_NET_WM_STATE"),
            NetWMStateModal: get_atom("_NET_WM_STATE_MODAL"),
            NetWMStateSticky: get_atom("_NET_WM_STATE_STICKY"),
            NetWMStateMaximizedVert: get_atom("_NET_WM_STATE_MAXIMIZED_VERT"),
            NetWMStateMaximizedHorz: get_atom("_NET_WM_STATE_MAXIMIZED_HORZ"),
            NetWMStateShaded: get_atom("_NET_WM_STATE_SHADED"),
            NetWMStateSkipTaskbar: get_atom("_NET_WM_STATE_SKIP_TASKBAR"),
            NetWMStateSkipPager: get_atom("_NET_WM_STATE_SKIP_PAGER"),
            NetWMStateHidden: get_atom("_NET_WM_STATE_HIDDEN"),
            NetWMStateFullscreen: get_atom("_NET_WM_STATE_FULLSCREEN"),
            NetWMStateAbove: get_atom("_NET_WM_STATE_ABOVE"),
            NetWMStateBelow: get_atom("_NET_WM_STATE_BELOW"),
            NetWMStateDemandsAttention: get_atom("_NET_WM_STATE_DEMANDS_ATTENTION"),

            NetWMAction: get_atom("_NET_WM_ALLOWED_ACTIONS"),
            NetWMActionMove: get_atom("_NET_WM_ACTION_MOVE"),
            NetWMActionResize: get_atom("_NET_WM_ACTION_RESIZE"),
            NetWMActionMinimize: get_atom("_NET_WM_ACTION_MINIMIZE"),
            NetWMActionShade: get_atom("_NET_WM_ACTION_SHADE"),
            NetWMActionStick: get_atom("_NET_WM_ACTION_STICK"),
            NetWMActionMaximizeHorz: get_atom("_NET_WM_ACTION_MAXIMIZE_HORZ"),
            NetWMActionMaximizeVert: get_atom("_NET_WM_ACTION_MAXIMIZE_VERT"),
            NetWMActionFullscreen: get_atom("_NET_WM_ACTION_FULLSCREEN"),
            NetWMActionChangeDesktop: get_atom("_NET_WM_ACTION_CHANGE_DESKTOP"),
            NetWMActionClose: get_atom("_NET_WM_ACTION_CLOSE"),

            NetWMWindowType: get_atom("_NET_WM_WINDOW_TYPE"),
            NetWMWindowTypeDesktop: get_atom("_NET_WM_WINDOW_TYPE_DESKTOP"),
            NetWMWindowTypeDock: get_atom("_NET_WM_WINDOW_TYPE_DOCK"),
            NetWMWindowTypeToolbar: get_atom("_NET_WM_WINDOW_TYPE_TOOLBAR"),
            NetWMWindowTypeMenu: get_atom("_NET_WM_WINDOW_TYPE_MENU"),
            NetWMWindowTypeUtility: get_atom("_NET_WM_WINDOW_TYPE_UTILITY"),
            NetWMWindowTypeSplash: get_atom("_NET_WM_WINDOW_TYPE_SPLASH"),
            NetWMWindowTypeDialog: get_atom("_NET_WM_WINDOW_TYPE_DIALOG"),
            NetSupportingWmCheck: get_atom("_NET_SUPPORTING_WM_CHECK"),

            NetClientList: get_atom("_NET_CLIENT_LIST"),
            NetDesktopViewport: get_atom("_NET_DESKTOP_VIEWPORT"),
            NetNumberOfDesktops: get_atom("_NET_NUMBER_OF_DESKTOPS"),
            NetCurrentDesktop: get_atom("_NET_CURRENT_DESKTOP"),
            NetDesktopNames: get_atom("_NET_DESKTOP_NAMES"),
            NetWMDesktop: get_atom("_NET_WM_DESKTOP"),
            NetWMStrutPartial: get_atom("_NET_WM_DESKTOP"),
            NetWMStrut: get_atom("_NET_WM_STRUT"),

            UTF8String: get_atom("UTF8_STRING"),
        }
    }
}

impl<'a, C: Connection> WM<'a, C> {
    pub fn new(connection: &'a C, screen_num: usize) -> Result<WM<'a, C>, XlibError> {
        tracing::info!("Opening Xlib");
        let xlib = Xlib::open().expect("open xft");
        // let xft = Xft::open().expect("open xft");

        // SAFETY:
        //   - passing NULL as the argument here is valid as documented here: https://man.archlinux.org/man/extra/libx11/XOpenDisplay.3.en
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };

        let setup = connection.setup();

        let screen = &setup.roots[screen_num];

        let win_id = connection.generate_id()?;

        // TODO: idk what to do with this

        // let pixmap = connection.create_pixmap(
        //     COPY_DEPTH_FROM_PARENT,
        //     win_id,
        //     screen.root,
        //     screen.width_in_pixels,
        //     screen.height_in_pixels,
        // )?;

        let black_gc = connection.generate_id()?;
        let font = connection.generate_id()?;
        connection.open_font(font, b"9x15")?;

        let gc_aux = CreateGCAux::new()
            .graphics_exposures(0)
            .background(screen.white_pixel)
            .foreground(screen.black_pixel)
            .font(font);
        connection.create_gc(black_gc, screen.root, &gc_aux)?;
        connection.close_font(font)?;

        unsafe {
            // let graphics_context =
            //     (xlib.XCreateGC)(display, screen.root.into(), 0, std::ptr::null_mut());

            let normal = (xlib.XCreateFontCursor)(display, LEFT_PTR);
            let resize = (xlib.XCreateFontCursor)(display, SIZING);
            let r#move = (xlib.XCreateFontCursor)(display, FLEUR);

            Ok(WM {
                xlib,
                connection: connection,
                display,
                // graphics_context,
                cursors: Cursors {
                    normal,
                    resize,
                    r#move,
                },
                fonts: vec![],
                screen_num,
                windows: vec![],
                black_gc,
                sequences_to_ignore: Default::default(),
            })
        }
    }
}

#[derive(Default)]
pub struct Xywh {
    x: i32,
    y: i32,
    h: i32,
    w: i32,
    minw: i32,
    maxw: i32,
    minh: i32,
    maxh: i32,
}

impl Xywh {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            h: height,
            w: width,
            ..Default::default()
        }
    }
}

#[derive(Default)]
pub struct BoundingBox {
    x: i32,
    pub y: i32,
    pub height: i32,
    width: i32,
}

impl BoundingBox {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            height,
            width,
        }
    }
}

pub struct Monitor<'a, C: Connection> {
    pub root: xlib::Window,
    pub output: String,
    pub bounding_box: BoundingBox,
    pub bar: Bar<'a, C>,
    _phantom_data: PhantomData<&'a C>,
}

impl<'a, C: Connection> Monitor<'a, C> {
    pub fn get_monitors(wm: &WM<'a, C>) -> Result<Vec<Monitor<'a, C>>, XlibError> {
        let root = &wm.connection.setup().roots[wm.screen_num];
        let monitors = wm
            .connection
            .randr_get_monitors(root.root, true)?
            .reply()?
            .monitors;

        monitors
            .iter()
            .map(|m| {
                let output_info = wm
                    .connection
                    .randr_get_output_info(
                        *m.outputs.iter().next().expect("monitor output"),
                        CURRENT_TIME,
                    )?
                    .reply()?;

                let crtc = wm
                    .connection
                    .randr_get_crtc_info(output_info.crtc, CURRENT_TIME)?
                    .reply()?;

                let mut monitor = Monitor::with_bbox(
                    BoundingBox::new(
                        crtc.x.into(),
                        crtc.y.into(),
                        crtc.width as i32,
                        crtc.height as i32,
                    ),
                    30,
                );

                monitor.root = root.root.into();
                monitor.output = String::from_utf8(output_info.name).expect("output name utf8");

                Ok(monitor)
            })
            .collect()
    }
    pub fn update_bar(&mut self, drawable: &WM<'a, C>) -> Result<(), XlibError> {
        unsafe {
            let mut attrs: XSetWindowAttributes = std::mem::zeroed();
            attrs.override_redirect = true.into();
            attrs.background_pixmap = ParentRelative as u64;
            attrs.event_mask = ButtonPressMask | ExposureMask;

            if self.bar.window.is_some() {
                return Ok(());
            }

            let win_id = drawable.connection.generate_id().expect("win id");

            let root = &drawable.connection.setup().roots[drawable.screen_num];

            let resource_db = x11rb::resource_manager::new_from_default(drawable.connection)?;

            let cursor_handle =
                CursorHandle::new(drawable.connection, drawable.screen_num, &resource_db)?
                    .reply()?;

            let cursor = cursor_handle.load_cursor(drawable.connection, "left_ptr")?;

            let window_aux = CreateWindowAux::new()
                .event_mask(EventMask::BUTTON_PRESS | EventMask::EXPOSURE)
                .override_redirect(Some(true.into()))
                .background_pixel(root.white_pixel)
                .cursor(cursor);

            let bar_window = drawable.connection.create_window(
                COPY_DEPTH_FROM_PARENT,
                win_id,
                root.root,
                self.bounding_box.x.try_into().unwrap(),
                self.bar.y.try_into().unwrap(),
                self.bounding_box.width.try_into().unwrap(),
                self.bar.height.try_into().unwrap(),
                0,
                WindowClass::COPY_FROM_PARENT,
                root.root_visual,
                &window_aux,
            )?;

            self.bar.window = Some(win_id.into());

            let atom = drawable
                .connection
                .intern_atom(false, "WM_CLASS".as_bytes())?
                .reply()?;

            drawable.connection.map_window(win_id)?;
            // drawable.connection.change_property(PropMode::REPLACE, win_id, atom.atom, AtomEnum::ATOM, , , );
            drawable.connection.change_property8(
                PropMode::REPLACE,
                win_id,
                AtomEnum::WM_NAME,
                AtomEnum::STRING,
                "Bunnuafeth bar".as_bytes(),
            )?;
            drawable.connection.change_property8(
                PropMode::REPLACE,
                win_id,
                AtomEnum::WM_CLASS,
                AtomEnum::STRING,
                "bunnuafeth-bar".as_bytes(),
            )?;

            // (xlib.XDefineCursor)(drawable.display, bar_window, drawable.cursors.normal);
            // (xlib.XMapRaised)(drawable.display, bar_window);
        }
        Ok(())
    }
    pub fn draw_bar(&mut self, xlib: &Xlib, drawable: &WM<'a, C>) {
        // self.bar.update_status(xlib, drawable);

        if !self.bar.show {
            return;
        }
    }
}

pub struct Bar<'a, C: Connection> {
    pub window: Option<Window>,
    pub show: bool,
    pub pos: BarPosition,
    pub y: i32,
    pub status_text: String,
    height: i32,
    _phantom_data: PhantomData<&'a C>,
}

impl<'a, C: Connection> Bar<'a, C> {
    pub fn update_position(&self, drawable: &WM<'a, C>) -> Result<(), XlibError> {
        let mut monitors = Monitor::get_monitors(drawable)?;

        for monitor in monitors.iter_mut() {
            if monitor.bar.show {
                monitor.bounding_box.height -= self.height;
                match monitor.bar.pos {
                    BarPosition::Top => {
                        monitor.bar.y = monitor.bounding_box.y;
                        monitor.bounding_box.y += self.height;
                    }
                    BarPosition::Bottom => {
                        monitor.bar.y = monitor.bounding_box.height + monitor.bounding_box.y;
                    }
                }
            } else {
                monitor.bar.y = -self.height;
            }
        }

        Ok(())
    }

    // pub fn update_status(&mut self, xlib: &Xlib, wm: &WM) {
    //     let root = &wm.connection.setup().roots[wm.screen_num];

    //     wm.connection.get_property(false, root.root);

    //     if let Ok(status) = get_text_prop(xlib, wm, root.root.into(), xlib::XA_WM_NAME) {
    //         self.status_text = status;
    //     }
    // }
}

pub enum BarPosition {
    Top,
    Bottom,
}

impl<'a, C: Connection> Monitor<'a, C> {
    pub fn new(bar_height: i32) -> Self {
        Self {
            output: String::new(),
            bounding_box: BoundingBox::new(0, 0, 0, 0),
            root: Default::default(),
            bar: Bar {
                window: None,
                show: true,
                pos: BarPosition::Top,
                y: 0,
                status_text: String::new(),
                height: bar_height,
                _phantom_data: PhantomData,
            },
            _phantom_data: PhantomData,
        }
    }
    pub fn with_bbox(bounding_box: BoundingBox, bar_height: i32) -> Self {
        Self {
            output: String::new(),
            bounding_box,
            root: Default::default(),
            bar: Bar {
                window: None,
                show: true,
                pos: BarPosition::Top,
                y: 0,
                status_text: String::new(),
                height: bar_height,
                _phantom_data: PhantomData,
            },
            _phantom_data: PhantomData,
        }
    }
}

pub fn run(connection: &RustConnection) -> Result<(), XlibError> {
    loop {
        connection.flush()?;
        let event = connection.wait_for_event()?;

        let mut event_option = Some(event);

        while let Some(event) = event_option {
            // if let x11rb::protocol::Event::ClientMessage(_) = event {
            //     // This is start_timeout_thread() signaling us to close (most likely).
            //     return Ok(());
            // }

            println!("event: {:#?}", event);

            // wm_state.handle_event(event)?;
            event_option = connection.poll_for_event()?;
        }
    }

    #[allow(unreachable_code)]
    Ok(())
}

pub fn setup_wm_attrs<'a, C: Connection>(wm: &WM<'a, C>) -> Result<(), XlibError> {
    let screen = &wm.connection.setup().roots[wm.screen_num];

    let resource_db = x11rb::resource_manager::new_from_default(wm.connection)?;

    let cursor_handle = CursorHandle::new(wm.connection, wm.screen_num, &resource_db)?.reply()?;

    let cursor = cursor_handle.load_cursor(wm.connection, "left_ptr")?;

    let change = ChangeWindowAttributesAux::default()
        .event_mask(
            EventMask::SUBSTRUCTURE_REDIRECT
                | EventMask::SUBSTRUCTURE_NOTIFY
                | EventMask::BUTTON_PRESS
                | EventMask::POINTER_MOTION
                | EventMask::STRUCTURE_NOTIFY,
        )
        .cursor(cursor);

    let res = wm
        .connection
        .change_window_attributes(screen.root, &change)?
        .check();

    if let Err(ReplyError::X11Error(ref error)) = res {
        if error.error_kind == ErrorKind::Access {
            eprintln!("Another WM is already running.");
            std::process::exit(1);
        }
    }

    let atoms = Atoms::new(wm.connection)?.reply()?;

    let create_window = CreateWindowAux::new();

    let win_id = wm.connection.generate_id()?;

    wm.connection
        .bunnu_create_simple_window(win_id, screen.root, 0, 0, 1, 1, 0, &create_window)?;

    wm.connection.change_property32(
        PropMode::REPLACE,
        win_id,
        atoms._NET_SUPPORTING_WM_CHECK,
        AtomEnum::WINDOW,
        &[win_id],
    )?;

    wm.connection.change_property8(
        PropMode::REPLACE,
        win_id,
        atoms._NET_WM_NAME,
        AtomEnum::STRING,
        "Bunnuafeth".as_bytes(),
    )?;

    wm.connection.change_property32(
        PropMode::REPLACE,
        screen.root,
        atoms._NET_SUPPORTING_WM_CHECK,
        AtomEnum::WINDOW,
        &[win_id],
    )?;

    let net_supported = atoms.net_supported();

    wm.connection.change_property32(
        PropMode::REPLACE,
        screen.root,
        atoms._NET_SUPPORTED,
        AtomEnum::ATOM,
        &net_supported,
    )?;

    wm.connection
        .delete_property(screen.root, atoms._NET_CLIENT_LIST)?;

    wm.connection
        .randr_select_input(screen.root, NotifyMask::SCREEN_CHANGE)?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum XlibError {
    #[error("failed to get status")]
    FailedStatus,

    #[error("no font loaded")]
    NoFontLoaded,

    #[error(transparent)]
    XrbConnectioError(#[from] x11rb::errors::ConnectionError),

    #[error(transparent)]
    XrbReplyOrIdError(#[from] x11rb::errors::ReplyOrIdError),

    #[error(transparent)]
    XrbReplyError(#[from] x11rb::errors::ReplyError),
}

fn get_text_prop<'a, C: Connection>(
    xlib: &Xlib,
    drawable: &WM<'a, C>,
    window: xlib::Window,
    atom: xlib::Atom,
) -> Result<String, XlibError> {
    unsafe {
        let mut text_prop: xlib::XTextProperty = std::mem::zeroed();
        let status: c_int = (xlib.XGetTextProperty)(drawable.display, window, &mut text_prop, atom);
        if status == 0 {
            return Err(XlibError::FailedStatus);
        }
        if let Ok(s) = CString::from_raw(text_prop.value.cast::<c_char>()).into_string() {
            return Ok(s);
        }
    };
    Err(XlibError::FailedStatus)
}
