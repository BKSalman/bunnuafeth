use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    io::{BufRead, BufReader},
    marker::PhantomData,
};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::{ConnectionExt as XrandrConnectionExt, NotifyMask},
        // xinput::KeyPressEvent,
        xproto::{
            Atom, AtomEnum, ButtonPressEvent, ButtonReleaseEvent, ChangeWindowAttributesAux,
            ClientMessageEvent, ConfigureWindowAux, ConnectionExt as XlibConnectionExt, CoordMode,
            CreateGCAux, CreateWindowAux, EnterNotifyEvent, EventMask, ExposeEvent, FontDraw,
            Gcontext, GetGeometryReply, GrabMode, KeyPressEvent, MapRequestEvent, MapState,
            ModMask, MotionNotifyEvent, Point, PropMode, Screen, SetMode, UnmapNotifyEvent, Window,
            WindowClass,
        },
        ErrorKind,
        Event,
    },
    rust_connection::ReplyError,
    wrapper::ConnectionExt as WrapperConnectionExt,
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, CURRENT_TIME,
};

pub const LEFT_PTR: u16 = 68;
pub const SIZING: u16 = 120;
pub const FLEUR: u16 = 52;
pub const HAND: u16 = 60;
pub const BAR_HEIGHT: u16 = 30;
pub const DRAG_BUTTON: u8 = 1;

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
    pub cursors: Cursors,
    pub fonts: Vec<Font>,
    pub screen_num: usize,
    pending_expose: HashSet<Window>,
    pub windows: Vec<WindowState>,
    pub black_gc: Gcontext,
    pub sequences_to_ignore: BinaryHeap<Reverse<u16>>,
    pub bar: Bar<'a, C>,
    drag_window: Option<(Window, (i16, i16))>,
    wm_protocols: Atom,
    wm_delete_window: Atom,
    config: Config,
    key_mapping: HashMap<KeyMapping, Command>,
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

        let wm_protocols = connection.intern_atom(false, b"WM_PROTOCOLS")?;
        let wm_delete_window = connection.intern_atom(false, b"WM_DELETE_WINDOW")?;

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
                status_text: String::new(),
                height: BAR_HEIGHT.try_into().unwrap(),
                _phantom_data: PhantomData,
            },
            pending_expose: Default::default(),
            drag_window: None,
            wm_protocols: wm_protocols.reply()?.atom,
            wm_delete_window: wm_delete_window.reply()?.atom,
            config,
            key_mapping: HashMap::new(),
        })
    }

    pub fn setup(&mut self) -> Result<(), XlibError> {
        let screen = self.screen();

        let change = ChangeWindowAttributesAux::default()
            .event_mask(
                EventMask::SUBSTRUCTURE_REDIRECT
                    | EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::KEY_PRESS
                    | EventMask::POINTER_MOTION
                    | EventMask::STRUCTURE_NOTIFY,
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

        let atoms = Atoms::new(self.connection)?.reply()?;

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

        self.connection.change_property32(
            PropMode::REPLACE,
            win_id,
            atoms._NET_SUPPORTING_WM_CHECK,
            AtomEnum::WINDOW,
            &[win_id],
        )?;

        self.connection.change_property8(
            PropMode::REPLACE,
            win_id,
            atoms._NET_WM_NAME,
            AtomEnum::STRING,
            "Bunnuafeth".as_bytes(),
        )?;

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_SUPPORTING_WM_CHECK,
            AtomEnum::WINDOW,
            &[win_id],
        )?;

        let net_supported = atoms.net_supported();

        self.connection.change_property32(
            PropMode::REPLACE,
            screen.root,
            atoms._NET_SUPPORTED,
            AtomEnum::ATOM,
            &net_supported,
        )?;

        self.connection
            .delete_property(screen.root, atoms._NET_CLIENT_LIST)?;

        self.connection
            .randr_select_input(screen.root, NotifyMask::SCREEN_CHANGE)?;

        self.connection.map_window(win_id)?;

        self.grab_hotkeys()?;

        Ok(())
    }

    fn get_keyboard_mapping(&self) -> Result<HashMap<KeyMapping, Command>, XlibError> {
        let setup = self.connection.setup();
        let lo = setup.min_keycode;
        let hi = setup.max_keycode;
        let capacity = hi - lo + 1;

        let mapping = self
            .connection
            .get_keyboard_mapping(lo, capacity)?
            .reply()?;

        let mut hotkeys = self.config.hotkeys.clone();
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

        Ok(map)
    }

    fn grab_hotkeys(&mut self) -> Result<(), XlibError> {
        self.key_mapping = self.get_keyboard_mapping()?;

        let screen = self.screen();

        self.key_mapping.keys().for_each(|hk| {
            self.connection
                .grab_key(
                    false,
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

    fn screen(&self) -> &Screen {
        &self.connection.setup().roots[self.screen_num]
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
            self.bar.y.try_into().unwrap(),
            1024, // self.bounding_box.width.try_into().unwrap(),
            self.bar.height.try_into().unwrap(),
            0,
            WindowClass::COPY_FROM_PARENT,
            root.root_visual,
            &window_aux,
        )?;

        self.bar.window = Some(bar_win_id.into());

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

        tracing::debug!("mapping bar {bar_win_id}");
        self.connection.map_window(bar_win_id)?;

        let geom = self.connection.get_geometry(bar_win_id)?.reply()?;

        self.windows
            .push(WindowState::new(bar_win_id, root.root, &geom));

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

    pub fn draw_bar(&self, state: &WindowState) -> Result<(), XlibError> {
        tracing::debug!("drawing bar");
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

        tracing::debug!("drawing text: {}", String::from_utf8_lossy(&reply.value));

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

    fn find_window_by_id(&self, win: Window) -> Option<&WindowState> {
        self.windows
            .iter()
            .find(|state| state.window == win || state.frame_window == win)
    }

    fn find_window_by_id_mut(&mut self, win: Window) -> Option<&mut WindowState> {
        self.windows
            .iter_mut()
            .find(|state| state.window == win || state.frame_window == win)
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

    fn manage_window(&mut self, win: Window, geom: &GetGeometryReply) -> Result<(), XlibError> {
        tracing::debug!("Managing window {:?}", win);
        let screen = self.screen();
        assert!(
            self.find_window_by_id(win).is_none(),
            "Unmanaged window should not exist already!"
        );

        let frame_win = self.connection.generate_id()?;
        let win_aux = CreateWindowAux::new()
            .event_mask(
                EventMask::EXPOSURE
                    | EventMask::SUBSTRUCTURE_NOTIFY
                    | EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::KEY_PRESS
                    | EventMask::KEY_RELEASE
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

        tracing::debug!("window geometry {geom:#?}");

        self.windows.push(WindowState::new(win, frame_win, geom));

        // Ignore all events caused by reparent_window(). All those events have the sequence number
        // of the reparent_window() request, thus remember its sequence number. The
        // grab_server()/ungrab_server() is done so that the server does not handle other clients
        // in-between, which could cause other events to get the same sequence number.
        self.sequences_to_ignore
            .push(Reverse(cookie.sequence_number() as u16));
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), XlibError> {
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

        // if !matches!(event, Event::MotionNotify(_)) {
        //     tracing::debug!("Got event {:?}", event);
        // }
        if should_ignore {
            tracing::debug!("  [ignored]");
            return Ok(());
        }
        match event {
            Event::UnmapNotify(event) => self.handle_unmap_notify(event),
            // protocol::Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::MapRequest(event) => {
                tracing::debug!("map request");
                self.handle_map_request(event)?
            }
            Event::Expose(event) => self.handle_expose(event),
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::ButtonPress(event) => self.handle_button_press(event),
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::MotionNotify(event) => self.handle_motion_notify(event)?,
            Event::KeyPress(event) => self.handle_key_press(event),
            _ => {}
        }

        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) {
        self.pending_expose.insert(event.window);
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> Result<(), XlibError> {
        tracing::debug!("handle map request");
        self.manage_window(
            event.window,
            &self.connection.get_geometry(event.window)?.reply()?,
        )
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), ReplyError> {
        if event.detail == DRAG_BUTTON {
            self.drag_window = None;
        }
        if let Some(state) = self.find_window_by_id(event.event) {
            if event.event_x >= state.close_x_position() {
                let event = ClientMessageEvent::new(
                    32,
                    state.window,
                    self.wm_protocols,
                    [self.wm_delete_window, 0, 0, 0, 0],
                );
                self.connection
                    .send_event(false, state.window, EventMask::NO_EVENT, event)?;
            }
        }
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

    fn handle_button_press(&mut self, event: ButtonPressEvent) {
        if event.detail != DRAG_BUTTON {
            return;
        }
        tracing::debug!("button 1 clicked");
        if let Some(state) = self.find_window_by_id(event.event) {
            if self.drag_window.is_none() && event.event_x < state.close_x_position() {
                let (x, y) = (-event.event_x, -event.event_y);
                self.drag_window = Some((state.frame_window, (x, y)));
            }
        }
    }

    fn refresh(&mut self) {
        while let Some(&win) = self.pending_expose.iter().next() {
            tracing::debug!("exposed window: {win}");
            self.pending_expose.remove(&win);
            if let Some(state) = self.find_window_by_id(win) {
                if let Err(err) = self.draw_bar(state) {
                    eprintln!(
                        "Error while redrawing window {:x?}: {:?}",
                        state.window, err
                    );
                }
            }
        }
    }

    fn handle_key_press(&self, event: KeyPressEvent) {
        let key_mapping = KeyMapping {
            code: event.detail,
            mods: u16::from(event.state) - 16,
        };
        if let Some(hotkey) = self.key_mapping.get(&key_mapping) {
            tracing::debug!("got hotkey: {:?}", hotkey);
        }
    }

    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) {
        let root = self.screen().root;
        let conn = self.connection;
        self.windows.retain(|state| {
            if state.window != event.window {
                return true;
            }
            conn.change_save_set(SetMode::DELETE, state.window).unwrap();
            conn.reparent_window(state.window, root, state.x, state.y)
                .unwrap();
            conn.destroy_window(state.frame_window).unwrap();
            false
        });
    }

    fn handle_motion_notify(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        if let Some((win, (x, y))) = self.drag_window {
            let (x, y) = (x + event.root_x, y + event.root_y);
            // Sigh, X11 and its mixing up i16 and i32
            let (x, y) = (x as i32, y as i32);
            self.connection
                .configure_window(win, &ConfigureWindowAux::new().x(x).y(y))?;
        }
        Ok(())
    }

    fn handle_enter(&self, event: EnterNotifyEvent) -> Result<(), XlibError> {
        // TODO: add border when focusing window
        // let change = ChangeWindowAttributesAux::new().border_pixel(self.black_gc);
        // self.connection.change_window_attributes(event.event)
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Execute(String),
}

pub struct Config {
    pub hotkeys: Vec<Hotkey>,
}

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub modmask: ModMask,
    pub keysym: u32,
    pub command: Command,
}

impl Hotkey {
    #[must_use]
    pub fn new(modmask: ModMask, keysym: u32, command: Command) -> Self {
        Self {
            modmask,
            keysym,
            command,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct KeyMapping {
    pub code: u8,
    pub mods: u16,
}

impl KeyMapping {
    pub fn new(code: u8, mods: u16) -> Self {
        KeyMapping { code, mods }
    }
}

#[derive(Debug, Clone)]
pub struct WindowState {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    pub window: Window,
    frame_window: Window,
}

impl WindowState {
    fn new(window: Window, frame_window: Window, geom: &GetGeometryReply) -> WindowState {
        WindowState {
            window,
            frame_window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
        }
    }

    fn close_x_position(&self) -> i16 {
        std::cmp::max(0, self.width - BAR_HEIGHT) as _
    }
}

pub struct Font {
    xfont: x11rb::protocol::xproto::Font,
    height: i32,
}

type Cursor = u32;

pub struct Cursors {
    pub normal: Cursor,
    pub resize: Cursor,
    pub r#move: Cursor,
    pub hand: Cursor,
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
    pub root: Window,
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

pub fn run<'a, C: Connection>(mut wm: WM<'a, C>) -> Result<(), XlibError> {
    let mut output = std::process::Command::new("kitty").spawn().unwrap();
    std::thread::spawn(move || {
        if let Some(stdout) = output.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Some(Ok(line)) = lines.next() {
                tracing::debug!("{line}");
            }
        }
    });

    loop {
        wm.refresh();
        wm.connection.flush()?;

        let event = wm.connection.wait_for_event()?;
        let mut event_option = Some(event);
        while let Some(event) = event_option {
            // if let x11rb::protocol::Event::ClientMessage(_) = event {
            //     // This is start_timeout_thread() signaling us to close (most likely).
            //     return Ok(());
            // }

            wm.handle_event(event)?;
            event_option = wm.connection.poll_for_event()?;
        }
    }
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

    #[error("window not found")]
    WindowNotFound,
}
