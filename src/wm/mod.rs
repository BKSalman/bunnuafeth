use crate::{atoms::Atoms, ButtonMapping, WindowType, RGBA};
use core::marker::PhantomData;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    process::Command,
};
use x11rb::{
    connection::Connection,
    // TODO: use this
    properties::WmHints,
    protocol::{
        glx::Window,
        xproto::{
            AtomEnum, ButtonPressEvent, ButtonReleaseEvent, ChangeWindowAttributesAux, Circulate,
            ClientMessageEvent, ConfigureRequestEvent, ConfigureWindowAux, ConnectionExt,
            CreateGCAux, CreateWindowAux, Cursor, DestroyNotifyEvent, EnterNotifyEvent, EventMask,
            ExposeEvent, FontDraw, Gcontext, GetGeometryReply, GrabMode, InputFocus, KeyPressEvent,
            MapRequestEvent, MapState, ModMask, MotionNotifyEvent, PropMode, Rectangle, Screen,
            SetMode, StackMode, Timestamp, UnmapNotifyEvent, WindowClass,
        },
        ErrorKind, Event,
    },
    rust_connection::ReplyError,
    wrapper::ConnectionExt as WrapperConnectionExt,
    COPY_DEPTH_FROM_PARENT,
    CURRENT_TIME,
    NONE,
};

use crate::{
    util::CommandExt, Bar, BarPosition, BunnuConnectionExt, Config, KeyMapping, WMCommand,
    WindowState, XlibError, BAR_HEIGHT,
};

pub const LEFT_PTR: u16 = 68;
pub const SIZING: u16 = 120;
pub const FLEUR: u16 = 52;
pub const HAND: u16 = 60;
const BORDER_WIDTH: u32 = 5;

pub struct Cursors {
    pub normal: Cursor,
    pub resize: Cursor,
    pub r#move: Cursor,
    pub hand: Cursor,
}

pub struct Font {
    xfont: x11rb::protocol::xproto::Font,
    height: i32,
}

type WindowPosition = (i16, i16);
type WindowSize = (u16, u16);

pub struct WM<'a, C: Connection> {
    pub connection: &'a C,
    pub atoms: Atoms,
    pub cursors: Cursors,
    pub fonts: Vec<Font>,
    pub screen_num: usize,
    pending_expose: HashSet<Window>,
    pub windows: Vec<WindowState>,
    pub black_gc: Gcontext,
    pub sequences_to_ignore: BinaryHeap<Reverse<u16>>,
    pub bar: Bar<'a, C>,
    drag_window: Option<(Window, WindowPosition)>,
    resize_window: Option<(Window, (WindowSize, WindowPosition))>,
    pointer_grabbed: bool,
    config: Config,
    key_mapping: HashMap<KeyMapping, WMCommand>,
    button_mapping: HashMap<ButtonMapping, WMCommand>,
    focused_window: Option<WindowState>,
    last_timestamp: Timestamp,
}

impl<'a, C: Connection> WM<'a, C> {
    pub fn new(
        connection: &'a C,
        screen_num: usize,
        config: Config,
    ) -> Result<WM<'a, C>, XlibError> {
        tracing::info!("setting up bunnuafeth");
        let setup = connection.setup();

        let screen = &setup.roots[screen_num];

        let win_id = connection.generate_id()?;

        connection.create_pixmap(
            screen.root_depth,
            win_id,
            screen.root,
            screen.width_in_pixels,
            screen.height_in_pixels,
        )?;

        let black_gc = connection.generate_id()?;
        let font = connection.generate_id()?;

        if let Err(e) = connection.open_font(font, b"6x13")?.check() {
            tracing::error!("failed to open font {e}");
            println!("DIR  MIN  MAX EXIST DFLT PROP ASC DESC NAME");

            for reply in connection.list_fonts_with_info(u16::max_value(), b"*")? {
                let reply = reply?;

                let dir = if reply.draw_direction == FontDraw::LEFT_TO_RIGHT {
                    "-->"
                } else if reply.draw_direction == FontDraw::RIGHT_TO_LEFT {
                    "<--"
                } else {
                    "???"
                };

                let (min, max, indicator) = if reply.min_byte1 == 0 && reply.max_byte1 == 0 {
                    (reply.min_char_or_byte2, reply.max_char_or_byte2, ' ')
                } else {
                    (u16::from(reply.min_byte1), u16::from(reply.max_byte1), '*')
                };

                let all = if reply.all_chars_exist { "all" } else { "some" };

                let name = String::from_utf8_lossy(&reply.name);

                println!(
                    "{} {}{:3} {}{:3} {:>5} {:4} {:4} {:3} {:4} {}",
                    dir,
                    indicator,
                    min,
                    indicator,
                    max,
                    all,
                    reply.default_char,
                    reply.properties.len(),
                    reply.font_ascent,
                    reply.font_descent,
                    name
                );
            }
            std::process::exit(1);
        }

        let gc_aux = CreateGCAux::new()
            .graphics_exposures(0)
            .background(screen.white_pixel)
            .foreground(screen.black_pixel)
            .font(font);
        connection.create_gc(black_gc, screen.root, &gc_aux)?;
        connection.close_font(font)?;

        let font = connection.generate_id()?;
        connection.open_font(font, b"cursor")?;

        let normal = connection.generate_id()?;
        connection.create_glyph_cursor(
            normal,
            font,
            font,
            LEFT_PTR,
            LEFT_PTR + 1,
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )?;
        let resize = connection.generate_id()?;
        connection.create_glyph_cursor(
            resize,
            font,
            font,
            SIZING,
            SIZING + 1,
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )?;
        let r#move = connection.generate_id()?;
        connection.create_glyph_cursor(
            r#move,
            font,
            font,
            FLEUR,
            FLEUR + 1,
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )?;
        let hand = connection.generate_id()?;
        connection.create_glyph_cursor(
            hand,
            font,
            font,
            HAND,
            HAND + 1,
            0,
            0,
            0,
            u16::MAX,
            u16::MAX,
            u16::MAX,
        )?;

        Ok(WM {
            connection,
            cursors: Cursors {
                normal,
                resize,
                r#move,
                hand,
            },
            fonts: vec![],
            screen_num,
            windows: vec![],
            black_gc,
            sequences_to_ignore: Default::default(),
            bar: Bar {
                window: None,
                show: true,
                pos: BarPosition::Top,
                y: 0,
                x: 0,
                status_text: String::new(),
                height: BAR_HEIGHT,
                width: screen.width_in_pixels,
                _phantom_data: PhantomData,
            },
            pending_expose: Default::default(),
            drag_window: None,
            resize_window: None,
            config,
            key_mapping: HashMap::new(),
            focused_window: None,
            button_mapping: HashMap::new(),
            pointer_grabbed: false,
            atoms: Atoms::new(connection)?.reply()?,
            last_timestamp: CURRENT_TIME,
        })
    }

    pub fn setup(&mut self) -> Result<(), XlibError> {
        let screen = self.screen();

        let change = ChangeWindowAttributesAux::default()
            .event_mask(
                EventMask::SUBSTRUCTURE_REDIRECT
                    | EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE, // | EventMask::STRUCTURE_NOTIFY,
                                                 // | EventMask::POINTER_MOTION,
            )
            .cursor(self.cursors.normal);

        let res = self
            .connection
            .change_window_attributes(screen.root, &change)?
            .check();

        if let Err(ReplyError::X11Error(ref error)) = res {
            if error.error_kind == ErrorKind::Access {
                tracing::error!("Another WM is already running.");
                std::process::exit(1);
            }
        }

        self.add_ewmh_default().expect("EWMH compliance");

        self.focus_window(FocusWindow::Root(screen.root))?;

        self.key_mapping()?;
        self.button_mapping();

        self.grab_hotkeys()?;
        self.grab_buttons();

        Ok(())
    }

    fn add_ewmh_default(&self) -> Result<(), XlibError> {
        let screen = self.screen();

        let create_window = CreateWindowAux::new();

        let win_id = self.connection.generate_id()?;

        self.connection.bunnu_create_simple_window(
            win_id,
            screen.root,
            0,
            0,
            1,
            1,
            0,
            &create_window,
        )?;

        self.connection
            .change_property32(
                PropMode::REPLACE,
                win_id,
                self.atoms._NET_SUPPORTING_WM_CHECK,
                AtomEnum::WINDOW,
                &[win_id],
            )?
            .check()?;

        self.connection
            .change_property8(
                PropMode::REPLACE,
                win_id,
                self.atoms._NET_WM_NAME,
                AtomEnum::STRING,
                "Bunnuafeth".as_bytes(),
            )?
            .check()?;

        self.connection
            .change_property32(
                PropMode::REPLACE,
                screen.root,
                self.atoms._NET_SUPPORTING_WM_CHECK,
                AtomEnum::WINDOW,
                &[win_id],
            )?
            .check()?;

        let net_supported = self.atoms.net_supported();

        self.connection
            .change_property32(
                PropMode::REPLACE,
                screen.root,
                self.atoms._NET_SUPPORTED,
                AtomEnum::ATOM,
                &net_supported,
            )?
            .check()?;

        self.connection
            .delete_property(screen.root, self.atoms._NET_CLIENT_LIST)?;

        self.connection
            .change_property32(
                PropMode::REPLACE,
                screen.root,
                self.atoms._NET_NUMBER_OF_DESKTOPS,
                AtomEnum::CARDINAL,
                &[0],
            )?
            .check()?;

        self.connection
            .change_property32(
                PropMode::REPLACE,
                screen.root,
                self.atoms._NET_CURRENT_DESKTOP,
                AtomEnum::CARDINAL,
                &[0],
            )?
            .check()?;

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            self.atoms._NET_DESKTOP_VIEWPORT,
            AtomEnum::CARDINAL,
            &[0; 2],
        )?;

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            self.atoms._NET_DESKTOP_GEOMETRY,
            AtomEnum::CARDINAL,
            &[
                screen.width_in_pixels as u32,
                screen.height_in_pixels as u32,
            ],
        )?;

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            self.atoms._NET_WORKAREA,
            AtomEnum::CARDINAL,
            &[
                0,
                BAR_HEIGHT as u32, // make this modular
                screen.width_in_pixels as u32,
                screen.height_in_pixels as u32 - BAR_HEIGHT as u32,
            ],
        )?;

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            self.atoms._NET_ACTIVE_WINDOW,
            AtomEnum::CARDINAL,
            &[],
        )?;

        self.connection.map_window(win_id)?.check()?;

        Ok(())
    }

    fn key_mapping(&mut self) -> Result<(), XlibError> {
        let setup = self.connection.setup();
        let lo = setup.min_keycode;
        let hi = setup.max_keycode;
        let capacity = hi - lo + 1;

        let mapping = self
            .connection
            .get_keyboard_mapping(lo, capacity)?
            .reply()?;

        let mut hotkeys =
            self.config
                .hotkeys
                .clone()
                .into_iter()
                .fold(Vec::new(), |mut acc, hk| {
                    // grab the buttons with and without numlock (Mod2) and capslock.
                    let mods = vec![
                        hk.modmask,
                        hk.modmask | ModMask::M2,
                        hk.modmask | ModMask::LOCK,
                    ];
                    for r#mod in mods {
                        let mut hk = hk.clone();
                        hk.modmask = r#mod;
                        acc.push(hk);
                    }
                    acc
                });
        let mut map = HashMap::new();

        // self.config.hotkeys.keys()
        for (keysym_ind, sym) in mapping.keysyms.iter().enumerate() {
            while let Some(keymap_ind) = hotkeys.iter().position(|k| &k.keysym == sym) {
                let key_def = hotkeys.swap_remove(keymap_ind);
                let mods = key_def.modmask;
                let modded_ind: usize = keysym_ind + Into::<u32>::into(mods) as usize;
                let code: usize = (modded_ind - Into::<u32>::into(mods) as usize)
                    / mapping.keysyms_per_keycode as usize
                    + lo as usize;
                let key = KeyMapping::new(code as u8, mods.into());
                map.insert(key, key_def.command);
            }
        }

        self.key_mapping = map;
        Ok(())
    }

    fn grab_hotkeys(&mut self) -> Result<(), XlibError> {
        let screen = self.screen();

        self.key_mapping.keys().for_each(|hk| {
            self.connection
                .grab_key(
                    true,
                    screen.root,
                    hk.mods.into(),
                    hk.code,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .unwrap()
                .check()
                .unwrap();
        });

        Ok(())
    }

    fn button_mapping(&mut self) {
        self.button_mapping =
            self.config
                .mouse_hotkeys
                .iter()
                .fold(HashMap::new(), |mut acc, mhk| {
                    [mhk.mods, mhk.mods | ModMask::M2, mhk.mods | ModMask::LOCK]
                        .into_iter()
                        .for_each(|r#mod| {
                            acc.insert(ButtonMapping::new(mhk.button, r#mod), mhk.command.clone());
                        });
                    acc
                });
    }

    fn grab_buttons(&self) {
        let screen = self.screen();
        self.button_mapping.keys().for_each(|m| {
            self.connection
                .grab_button(
                    false,
                    screen.root,
                    EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                    NONE,
                    NONE,
                    m.button,
                    m.mods,
                )
                .unwrap()
                .check()
                .unwrap();
        })
    }

    pub fn create_bar(&mut self) -> Result<(), XlibError> {
        let bar_win_id = self.connection.generate_id()?;

        let root = &self.connection.setup().roots[self.screen_num];

        let window_aux = CreateWindowAux::new()
            .event_mask(EventMask::BUTTON_PRESS | EventMask::EXPOSURE)
            .override_redirect(Some(true.into()))
            .background_pixel(root.white_pixel)
            .cursor(self.cursors.hand);

        self.connection.create_window(
            COPY_DEPTH_FROM_PARENT,
            bar_win_id,
            root.root,
            0, // self.bounding_box.x.try_into().unwrap(),
            self.bar.y,
            self.screen().width_in_pixels, // self.bounding_box.width.try_into().unwrap(),
            self.bar.height,
            0,
            WindowClass::COPY_FROM_PARENT,
            root.root_visual,
            &window_aux,
        )?;

        self.bar.window = Some(bar_win_id);

        self.connection.change_property8(
            PropMode::REPLACE,
            bar_win_id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            "Bunnuafeth bar".as_bytes(),
        )?;
        self.connection.change_property8(
            PropMode::REPLACE,
            bar_win_id,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            "bunnuafeth-bar".as_bytes(),
        )?;

        self.connection.change_property32(
            PropMode::REPLACE,
            bar_win_id,
            self.atoms._NET_WM_WINDOW_TYPE,
            AtomEnum::ATOM,
            &[self.atoms._NET_WM_WINDOW_TYPE_DOCK],
        )?;

        tracing::debug!("mapping bar {bar_win_id}");
        self.connection.map_window(bar_win_id)?;

        let geom = self.connection.get_geometry(bar_win_id)?.reply()?;

        self.windows
            .push(WindowState::new(bar_win_id, &geom, true, WindowType::Dock));

        Ok(())
    }

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

    pub fn draw_bar(&self) -> Result<(), XlibError> {
        if let Some(bar_window) = self.bar.window {
            self.connection
                .poly_fill_rectangle(
                    bar_window,
                    self.black_gc,
                    &[Rectangle {
                        x: self.bar.x,
                        y: self.bar.y,
                        width: self.bar.width,
                        height: self.bar.height,
                    }],
                )?
                .check()?;
            if let Some(fw_state) = &self.focused_window {
                let reply = self
                    .connection
                    .get_property(
                        false,
                        fw_state.window,
                        AtomEnum::WM_NAME,
                        AtomEnum::STRING,
                        0,
                        std::u32::MAX,
                    )?
                    .reply()?;
                self.connection
                    .image_text8(bar_window, self.black_gc, 1, 10, &reply.value)?
                    .check()?;
            } else {
                self.connection
                    .image_text8(bar_window, self.black_gc, 1, 10, b"something")?;
            }
        }

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

    fn find_window_by_id(&self, win: Window) -> Option<&WindowState> {
        self.windows.iter().find(|state| state.window == win)
    }

    fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut WindowState> {
        self.windows.iter_mut().find(|state| state.window == win)
    }

    /// Scan for already existing windows and manage them
    pub fn scan_windows(&mut self) -> Result<(), XlibError> {
        // Get the already existing top-level windows.
        let screen = self.screen();
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

    fn manage_window(&mut self, window: Window, geom: &GetGeometryReply) -> Result<(), XlibError> {
        tracing::debug!("managing window {:?}", window);
        let screen = self.screen();
        assert!(
            self.find_window_by_id(window).is_none(),
            "Unmanaged window should not exist already!"
        );
        let change = ChangeWindowAttributesAux::new().event_mask(
            EventMask::ENTER_WINDOW
                | EventMask::FOCUS_CHANGE
                | EventMask::PROPERTY_CHANGE
                | EventMask::VISIBILITY_CHANGE
                | EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY,
        );
        let cookie = self.connection.change_window_attributes(window, &change)?;

        let configure = ConfigureWindowAux::new().border_width(BORDER_WIDTH);

        self.connection
            .configure_window(window, &configure)?
            .check()
            .unwrap();

        self.connection.change_property32(
            PropMode::REPLACE,
            window,
            self.atoms._NET_FRAME_EXTENTS,
            AtomEnum::CARDINAL,
            // [left, right, top, bottom]
            &[BORDER_WIDTH; 4],
        )?;

        self.connection.grab_server()?;
        self.connection.change_save_set(SetMode::INSERT, window)?;
        self.connection.change_property32(
            PropMode::APPEND,
            screen.root,
            self.atoms._NET_CLIENT_LIST,
            AtomEnum::WINDOW,
            &[window],
        )?;
        self.connection.map_window(window)?.sequence_number();
        self.connection.ungrab_server()?;

        let window_type = self.get_window_type(window)?;

        let window_state = WindowState::new(
            window,
            geom,
            false,
            window_type.unwrap_or(WindowType::Normal),
        );

        self.focus_window(FocusWindow::Normal(Some(&window_state)))?;
        self.windows.push(window_state);

        // Ignore all events caused by reparent_window(). All those events have the sequence number
        // of the reparent_window() request, thus remember its sequence number. The
        // grab_server()/ungrab_server() is done so that the server does not handle other clients
        // in-between, which could cause other events to get the same sequence number.
        self.sequences_to_ignore
            .push(Reverse(cookie.sequence_number() as u16));
        Ok(())
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> Result<(), XlibError> {
        let mut should_ignore = false;
        if let Some(seqno) = event.wire_sequence_number() {
            // Check sequences_to_ignore and remove entries with old (=smaller) numbers.
            while let Some(&Reverse(to_ignore)) = self.sequences_to_ignore.peek() {
                // Sequence numbers can wrap around, so we cannot simply check for
                // "to_ignore <= seqno". This is equivalent to "to_ignore - seqno <= 0", which is what we
                // check instead. Since sequence numbers are unsigned, we need a trick: We decide
                // that values from [MAX/2, MAX] count as "<= 0" and the rest doesn't.
                if to_ignore.wrapping_sub(seqno) <= u16::max_value() / 2 {
                    // If the two sequence numbers are equal, this event should be ignored.
                    should_ignore = to_ignore == seqno;
                    break;
                }
                self.sequences_to_ignore.pop();
            }
        }

        if !matches!(event, Event::MotionNotify(_)) {
            tracing::debug!("got event {:?}", event);
        }
        if should_ignore {
            tracing::debug!("[ignored]");
            return Ok(());
        }
        match event {
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::Expose(event) => self.handle_expose(event),
            Event::DestroyNotify(event) => self.handle_destroy_notify(event)?,
            Event::UnmapNotify(event) => self.handle_unmap_notify(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::LeaveNotify(event) => self.handle_leave(event)?,
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(event)?,
            Event::KeyPress(event) => self.handle_key_press(event)?,
            _ => {}
        }

        Ok(())
    }

    fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> Result<(), ReplyError> {
        // Allow clients to change everything, except sibling / stack mode
        let aux = ConfigureWindowAux::from_configure_request(&event)
            .sibling(None)
            .stack_mode(None);
        tracing::debug!("configure: {:?}", aux);
        self.connection.configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) {
        self.pending_expose.insert(event.window);
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> Result<(), XlibError> {
        self.manage_window(
            event.window,
            &self.connection.get_geometry(event.window)?.reply()?,
        )
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), XlibError> {
        let button_mapping = ButtonMapping::new(event.detail, u16::from(event.state));

        if let Some(command) = self.button_mapping.get(&button_mapping) {
            match command {
                WMCommand::Execute(_) => todo!(),
                WMCommand::CloseWindow => todo!(),
                WMCommand::MoveWindow => {
                    tracing::debug!("move event: {}", event.child);
                    if let Some(win_state) = self.find_window_by_id(event.child) {
                        let window = win_state.window;
                        self.conditionally_grab_pointer(window)?;

                        let change = ChangeWindowAttributesAux::new().cursor(self.cursors.r#move);
                        self.connection.change_window_attributes(window, &change)?;
                        let geometry = self.connection.get_geometry(window)?.reply()?;
                        self.drag_window = Some((
                            window,
                            (geometry.x - event.event_x, geometry.y - event.event_y),
                        ));
                        self.raise_window(window)?;
                    }
                }
                WMCommand::ResizeWindow(_) => {
                    if let Some(win_state) = self.find_window_by_id(event.child) {
                        let window = win_state.window;
                        self.conditionally_grab_pointer(window)?;
                        let change = ChangeWindowAttributesAux::new().cursor(self.cursors.resize);
                        self.connection.change_window_attributes(window, &change)?;

                        let geometry = self.connection.get_geometry(window)?.reply()?;
                        self.resize_window = Some((
                            window,
                            (
                                (geometry.width, geometry.height),
                                (geometry.x - event.event_x, geometry.y - event.event_y),
                            ),
                        ));
                        self.raise_window(window)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle_button_release(&mut self, _event: ButtonReleaseEvent) -> Result<(), XlibError> {
        if let Some(drag_window) = self.drag_window {
            let change = ChangeWindowAttributesAux::new().cursor(self.cursors.normal);
            self.connection
                .change_window_attributes(drag_window.0, &change)?;
        }
        if let Some(resize_window) = self.resize_window {
            let change = ChangeWindowAttributesAux::new().cursor(self.cursors.normal);
            self.connection
                .change_window_attributes(resize_window.0, &change)?;
        }

        self.drag_window = None;
        self.resize_window = None;
        self.pointer_grabbed = false;
        self.connection.ungrab_pointer(CURRENT_TIME)?;
        Ok(())
    }

    fn handle_key_press(&mut self, event: KeyPressEvent) -> Result<(), XlibError> {
        let key_mapping = KeyMapping {
            code: event.detail,
            mods: u16::from(event.state),
        };

        if let Some(command) = self.key_mapping.get(&key_mapping) {
            match command {
                WMCommand::Execute(command) => {
                    // TODO: does this work like bash?
                    let mut command = command.split(' ');
                    if let Some(program) = command.next() {
                        if let Err(e) = Command::new(program)
                            .with_args(command.collect::<Vec<&str>>())
                            .spawn()
                        {
                            tracing::error!("command failed: {e}");
                        }
                    }
                }
                WMCommand::CloseWindow => {
                    if let Some(state) = &self
                        .focused_window
                        .as_ref()
                        .and_then(|fw| self.find_window_by_id(fw.window))
                    {
                        if state.window == self.screen().root {
                            return Ok(());
                        }

                        self.send_delete(state.window)?;
                    }
                }
                WMCommand::MoveWindow => {
                    let change = ChangeWindowAttributesAux::new().cursor(self.cursors.r#move);
                    self.connection
                        .change_window_attributes(event.event, &change)?;
                    if let Some(state) = self.find_window_by_id(event.event) {
                        if self.drag_window.is_none() {
                            let (x, y) = (-event.event_x, -event.event_y);
                            self.drag_window = Some((state.window, (x, y)));
                        }
                    }
                }
                WMCommand::ResizeWindow(_factor) => todo!(),
            };
        }

        Ok(())
    }

    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> Result<(), XlibError> {
        let root = self.screen().root;
        self.focus_window(FocusWindow::Root(root))?;
        self.windows.retain(|state| {
            if state.window != event.window {
                return true;
            }
            self.connection
                .change_save_set(SetMode::DELETE, state.window)
                .unwrap();
            self.connection
                .reparent_window(state.window, root, state.x, state.y)
                .unwrap();
            false
        });

        Ok(())
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        // limit the amount of requests for less CPU usage
        if event.time - self.last_timestamp <= (1000 / 60) {
            return Ok(());
        }
        self.last_timestamp = event.time;

        if let Some((win, (x, y))) = self.drag_window {
            let (x, y) = (x + event.root_x, y + event.root_y);
            self.connection
                .configure_window(win, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
            if let Some(state) = self.find_window_by_id_mut(win) {
                state.x = x;
                state.y = y;
            }
        } else if let Some((win, ((width, height), (x, y)))) = self.resize_window {
            let (width, height) = (
                width as i16 + x + event.event_x,
                height as i16 + y + event.event_y,
            );
            self.connection.configure_window(
                win,
                &ConfigureWindowAux::new()
                    .width(width as u32)
                    .height(height as u32),
            )?;
            if let Some(state) = self.find_window_by_id_mut(win) {
                state.width = width as u16;
                state.height = height as u16;
            }
        }
        Ok(())
    }

    fn handle_enter(&mut self, event: EnterNotifyEvent) -> Result<(), XlibError> {
        // TODO: can I remove this clone?
        let win = self.find_window_by_id(event.event).cloned();
        tracing::debug!("focusing {win:?}");
        self.focus_window(FocusWindow::Normal(win.as_ref()))?;
        // TODO: add border when focusing window
        // let change = ChangeWindowAttributesAux::new().border_pixel(self.black_gc);
        // self.connection.change_window_attributes(event.event)
        Ok(())
    }

    fn handle_leave(&mut self, event: EnterNotifyEvent) -> Result<(), XlibError> {
        if let Some((win, focused_window)) = self
            .find_window_by_id(event.event)
            .zip(self.focused_window.as_ref())
        {
            if focused_window.window == win.window {
                tracing::debug!("unfocusing {win:?} and focusing root window");
                let root = self.screen().root;
                self.focus_window(FocusWindow::Root(root)).unwrap();
            }
        }

        Ok(())
    }

    fn handle_destroy_notify(&mut self, event: DestroyNotifyEvent) -> Result<(), XlibError> {
        let root = self.screen().root;
        self.windows.retain(|state| {
            if state.window != event.window {
                return true;
            }
            self.connection
                .change_save_set(SetMode::DELETE, state.window)
                .unwrap();
            self.connection
                .reparent_window(state.window, root, state.x, state.y)
                .unwrap();
            // self.connection.destroy_window(state.frame_window).unwrap();
            false
        });

        let managed: Vec<_> = self.windows.iter().map(|w| w.window).collect();

        self.connection.change_property32(
            PropMode::REPLACE,
            self.screen().root,
            self.atoms._NET_CLIENT_LIST,
            AtomEnum::WINDOW,
            managed.as_slice(),
        )?;

        if let Some(fw) = &self.focused_window {
            if fw.window == event.window {
                let next_window = self.next_window();

                // TODO: can I remove this clone?
                self.focus_window(FocusWindow::Normal(next_window.cloned().as_ref()))?;
            }
        }

        Ok(())
    }

    fn next_window(&self) -> Option<&WindowState> {
        // TODO: make this better
        self.windows.iter().filter(|w| !w.is_bar).next_back()
    }

    fn focus_window(&mut self, window: FocusWindow) -> Result<(), XlibError> {
        match window {
            FocusWindow::Normal(win) => {
                if let Some(win) = win {
                    let change =
                        ChangeWindowAttributesAux::new().border_pixel(RGBA::CYAN.as_argb_u32());

                    self.connection
                        .change_window_attributes(win.window, &change)?
                        .check()
                        .unwrap();
                    self.connection
                        .set_input_focus(InputFocus::NONE, win.window, CURRENT_TIME)?;
                }
                if let Some(fw) = &self.focused_window {
                    if win.is_some_and(|win| fw.window != win.window) || win.is_none() {
                        let change = ChangeWindowAttributesAux::new()
                            .border_pixel(RGBA::BLACK.as_argb_u32());
                        self.connection
                            .change_window_attributes(fw.window, &change)?;
                    }
                }
                self.focused_window = win.cloned();
            }
            FocusWindow::Root(window) => {
                self.connection
                    .set_input_focus(InputFocus::NONE, window, CURRENT_TIME)?;
                self.focused_window = None;
            }
        }

        self.connection.flush()?;

        Ok(())
    }

    pub fn set_background_color(&self, window: Window, color: u32) -> Result<(), XlibError> {
        let change = ChangeWindowAttributesAux::new().background_pixel(color);
        self.connection.change_window_attributes(window, &change)?;
        let window_state = self
            .find_window_by_id(window)
            .ok_or(XlibError::WindowNotFound)?;
        self.connection.clear_area(
            false,
            window,
            window_state.x,
            window_state.y,
            window_state.width,
            window_state.height,
        )?;

        Ok(())
    }

    pub fn set_root_background_color(&self, color: u32) -> Result<(), XlibError> {
        let screen = self.screen();
        let change = ChangeWindowAttributesAux::new().background_pixel(color);
        self.connection
            .change_window_attributes(screen.root, &change)?;
        let root_geometry = self.connection.get_geometry(screen.root)?.reply()?;
        self.connection.clear_area(
            false,
            screen.root,
            root_geometry.x,
            root_geometry.y,
            root_geometry.width,
            root_geometry.height,
        )?;

        Ok(())
    }

    pub fn screen(&self) -> &Screen {
        &self.connection.setup().roots[self.screen_num]
    }

    pub(crate) fn refresh(&mut self) {
        while let Some(&win) = self.pending_expose.iter().next() {
            self.pending_expose.remove(&win);
            if let Some(state) = self.find_window_by_id(win) {
                if let Err(err) = self.draw_bar() {
                    tracing::debug!(
                        "Error while redrawing window {:x?}: {:?}",
                        state.window,
                        err
                    );
                }
            }
        }
    }

    fn send_delete(&self, window: Window) -> Result<(), XlibError> {
        let event = ClientMessageEvent::new(
            32,
            window,
            self.atoms.WM_PROTOCOLS,
            [self.atoms.WM_DELETE_WINDOW, 0, 0, 0, 0],
        );
        self.connection
            .send_event(false, window, EventMask::NO_EVENT, event)?;

        Ok(())
    }

    fn conditionally_grab_pointer(&mut self, window: Window) -> Result<(), XlibError> {
        if !self.pointer_grabbed {
            self.connection.grab_pointer(
                true,
                window,
                EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::BUTTON_MOTION
                    | EventMask::POINTER_MOTION,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                NONE,
                NONE,
                CURRENT_TIME,
            )?;
            self.pointer_grabbed = true;
        }

        Ok(())
    }

    fn raise_window(&mut self, window: Window) -> Result<(), XlibError> {
        let configure = ConfigureWindowAux::new().stack_mode(StackMode::ABOVE);
        self.connection.configure_window(window, &configure)?;

        if let Some(window_index) = self.windows.iter().position(|w| w.window == window) {
            if window_index == 0 {
                return Ok(());
            }
            let win_state = self.windows.remove(window_index);
            self.windows.insert(0, win_state);
        }

        Ok(())
    }

    fn get_window_type(&self, window: Window) -> Result<Option<WindowType>, XlibError> {
        let window_types = self
            .connection
            .get_property(
                false,
                window,
                self.atoms._NET_WM_WINDOW_TYPE,
                AtomEnum::ATOM,
                0,
                32 * 4,
            )?
            .reply()?;

        println!("format: {}", window_types.format);
        println!("value length: {}", window_types.value_len);

        let values = window_types.value32();

        if let Some(values) = values {
            Ok(values
                .map(|v| {
                    if v == self.atoms._NET_WM_WINDOW_TYPE_DESKTOP {
                        Some(WindowType::Desktop)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_DOCK {
                        Some(WindowType::Dock)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_TOOLBAR {
                        Some(WindowType::Toolbar)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_MENU {
                        Some(WindowType::Menu)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_UTILITY {
                        Some(WindowType::Utility)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_SPLASH {
                        Some(WindowType::Splash)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_DIALOG {
                        Some(WindowType::Dialog)
                    } else if v == self.atoms._NET_WM_WINDOW_TYPE_NORMAL {
                        Some(WindowType::Normal)
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or(None))
        } else {
            Ok(None)
        }
    }
}

enum FocusWindow<'a> {
    Normal(Option<&'a WindowState>),
    Root(Window),
}
