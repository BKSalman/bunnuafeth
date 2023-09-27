use atoms::Atoms;
use bar::{Bar, BarPosition};
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
};
use wm::WM;
use x11rb::protocol::{randr::ConnectionExt, xproto::ButtonIndex};
use x11rb::{
    connection::Connection,
    protocol::xproto::{GetGeometryReply, ModMask, Window},
    CURRENT_TIME,
};

mod atoms;
mod bar;
mod connection_wrapper;
pub mod layout;
mod util;
pub mod wm;

pub const BAR_HEIGHT: u16 = 30;

pub struct RGBA {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl RGBA {
    const CYAN: RGBA = RGBA::new(0x00, 0x55, 0x77, 0xff);
    const BLACK: RGBA = RGBA::new(0x00, 0x00, 0x00, 0xff);
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn as_argb_u32(&self) -> u32 {
        ((self.alpha as u32) << 24)
            | ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
    }

    pub fn as_rgba_u32(&self) -> u32 {
        ((self.red as u32) << 24)
            | ((self.green as u32) << 16)
            | ((self.blue as u32) << 8)
            | (self.alpha as u32)
    }
}

impl From<u32> for RGBA {
    fn from(value: u32) -> Self {
        let red = (value & 0x00_00_00_ff) as u8;
        let green = (value & 0x00_00_ff_00) as u8;
        let blue = (value & 0x00_ff_00_00) as u8;
        let alpha = (value & 0xff_00_00_00) as u8;

        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Into<u32> for RGBA {
    fn into(self) -> u32 {
        ((self.alpha as u32) << 24)
            | ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
    }
}

#[derive(Debug, Clone)]
pub enum WMCommand {
    Execute(String),
    CloseWindow,
    MoveWindow,
    /// the parameter here is not needed for mouse resizing
    ResizeWindow(i16),
    ToggleFullscreen,
}

pub struct Config {
    pub hotkeys: Vec<Hotkey>,
    pub mouse_hotkeys: Vec<MouseHotkey>,
}

#[derive(Debug, Clone)]
pub struct Hotkey {
    pub modmask: ModMask,
    pub keysym: u32,
    pub command: WMCommand,
}

impl Hotkey {
    #[must_use]
    pub fn new(modmask: ModMask, keysym: u32, command: WMCommand) -> Self {
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
pub struct MouseHotkey {
    pub command: WMCommand,
    pub mods: ModMask,
    pub button: ButtonIndex,
}

impl MouseHotkey {
    #[must_use]
    pub fn new(mods: ModMask, button: ButtonIndex, command: WMCommand) -> Self {
        Self {
            mods,
            button,
            command,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ButtonMapping {
    pub button: ButtonIndex,
    pub mods: ModMask,
}

impl ButtonMapping {
    pub fn new(button: impl Into<ButtonIndex>, mods: impl Into<ModMask>) -> Self {
        ButtonMapping {
            button: button.into(),
            mods: mods.into(),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct WindowProperties {
    is_fullscreen: bool,
    is_sticky: bool,
    is_modal: bool,
    is_maximized_horz: bool,
    is_maximized_vert: bool,
    is_shaded: bool,
    skip_taskbar: bool,
    skip_pager: bool,
    is_hidden: bool,
    above: bool,
    below: bool,
    demands_attention: bool,
}

impl WindowProperties {}

#[derive(Default, Debug, Clone)]
pub struct WindowPropertiesBuilder {
    is_fullscreen: Option<bool>,
    is_sticky: Option<bool>,
    is_modal: Option<bool>,
    is_maximized_horz: Option<bool>,
    is_maximized_vert: Option<bool>,
    is_shaded: Option<bool>,
    skip_taskbar: Option<bool>,
    skip_pager: Option<bool>,
    is_hidden: Option<bool>,
    above: Option<bool>,
    below: Option<bool>,
    demands_attention: Option<bool>,
}

impl WindowPropertiesBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_fullscreen(&mut self, is_fullscreen: bool) {
        self.is_fullscreen = Some(is_fullscreen);
    }

    pub fn is_modal(&mut self, is_modal: bool) {
        self.is_modal = Some(is_modal);
    }

    pub fn is_sticky(&mut self, is_sticky: bool) {
        self.is_sticky = Some(is_sticky);
    }

    pub fn is_maximized_horz(&mut self, is_maximized_horz: bool) {
        self.is_maximized_horz = Some(is_maximized_horz);
    }

    pub fn is_maximized_vert(&mut self, is_maximized_vert: bool) {
        self.is_maximized_vert = Some(is_maximized_vert);
    }

    pub fn is_shaded(&mut self, is_maximized_vert: bool) {
        self.is_maximized_vert = Some(is_maximized_vert);
    }

    pub fn skip_taskbar(&mut self, skip_taskbar: bool) {
        self.skip_taskbar = Some(skip_taskbar);
    }

    pub fn skip_pager(&mut self, skip_pager: bool) {
        self.skip_pager = Some(skip_pager);
    }

    pub fn is_hidden(&mut self, is_hidden: bool) {
        self.is_hidden = Some(is_hidden);
    }

    pub fn above(&mut self, above: bool) {
        self.above = Some(above);
    }

    pub fn below(&mut self, below: bool) {
        self.below = Some(below);
    }

    pub fn demands_attention(&mut self, demands_attention: bool) {
        self.demands_attention = Some(demands_attention);
    }

    pub fn build(self) -> WindowProperties {
        WindowProperties {
            is_fullscreen: self.is_fullscreen.unwrap_or_default(),
            is_sticky: self.is_sticky.unwrap_or_default(),
            is_modal: self.is_modal.unwrap_or_default(),
            is_maximized_horz: self.is_maximized_horz.unwrap_or_default(),
            is_maximized_vert: self.is_maximized_vert.unwrap_or_default(),
            is_shaded: self.is_shaded.unwrap_or_default(),
            skip_taskbar: self.skip_taskbar.unwrap_or_default(),
            skip_pager: self.skip_pager.unwrap_or_default(),
            is_hidden: self.is_hidden.unwrap_or_default(),
            above: self.above.unwrap_or_default(),
            below: self.below.unwrap_or_default(),
            demands_attention: self.demands_attention.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowState {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    pub window: Window,
    r#type: WindowType,
    properties: WindowProperties,
    is_floating: bool,
}

impl WindowState {
    fn new(
        window: Window,
        geom: &GetGeometryReply,
        r#type: WindowType,
        is_floating: bool,
    ) -> WindowState {
        WindowState {
            window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
            r#type,
            properties: WindowProperties::default(),
            is_floating: false,
        }
    }

    fn get_property_action(action: u32) -> Result<PropertyAction, XlibError> {
        // _NET_WM_STATE_REMOVE        0    /* remove/unset property */
        // _NET_WM_STATE_ADD           1    /* add/set property */
        // _NET_WM_STATE_TOGGLE        2    /* toggle property  */
        if action == 0 {
            Ok(PropertyAction::Remove)
        } else if action == 1 {
            Ok(PropertyAction::Add)
        } else if action == 2 {
            Ok(PropertyAction::Toggle)
        } else {
            Err(XlibError::InvalidAction)
        }
    }

    fn set_window_property(
        atoms: Atoms,
        property: u32,
        action: &PropertyAction,
        window_props: &mut WindowProperties,
    ) {
        match action {
            PropertyAction::Remove => {
                if property == atoms._NET_WM_STATE_MODAL {
                    window_props.is_modal = false;
                } else if property == atoms._NET_WM_STATE_STICKY {
                    window_props.is_sticky = false;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_VERT {
                    window_props.is_maximized_vert = false;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_HORZ {
                    window_props.is_maximized_horz = false;
                } else if property == atoms._NET_WM_STATE_SHADED {
                    window_props.is_shaded = false;
                } else if property == atoms._NET_WM_STATE_SKIP_TASKBAR {
                    window_props.skip_taskbar = false;
                } else if property == atoms._NET_WM_STATE_SKIP_PAGER {
                    window_props.skip_pager = false;
                } else if property == atoms._NET_WM_STATE_HIDDEN {
                    window_props.is_hidden = false;
                } else if property == atoms._NET_WM_STATE_FULLSCREEN {
                    window_props.is_fullscreen = false;
                } else if property == atoms._NET_WM_STATE_ABOVE {
                    window_props.above = false;
                } else if property == atoms._NET_WM_STATE_BELOW {
                    window_props.below = false;
                } else if property == atoms._NET_WM_STATE_DEMANDS_ATTENTION {
                    window_props.demands_attention = false;
                }
            }
            PropertyAction::Add => {
                if property == atoms._NET_WM_STATE_MODAL {
                    window_props.is_modal = true;
                } else if property == atoms._NET_WM_STATE_STICKY {
                    window_props.is_sticky = true;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_VERT {
                    window_props.is_maximized_vert = true;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_HORZ {
                    window_props.is_maximized_horz = true;
                } else if property == atoms._NET_WM_STATE_SHADED {
                    window_props.is_shaded = true;
                } else if property == atoms._NET_WM_STATE_SKIP_TASKBAR {
                    window_props.skip_taskbar = true;
                } else if property == atoms._NET_WM_STATE_SKIP_PAGER {
                    window_props.skip_pager = true;
                } else if property == atoms._NET_WM_STATE_HIDDEN {
                    window_props.is_hidden = true;
                } else if property == atoms._NET_WM_STATE_FULLSCREEN {
                    window_props.is_fullscreen = true;
                } else if property == atoms._NET_WM_STATE_ABOVE {
                    window_props.above = true;
                } else if property == atoms._NET_WM_STATE_BELOW {
                    window_props.below = true;
                } else if property == atoms._NET_WM_STATE_DEMANDS_ATTENTION {
                    window_props.demands_attention = true;
                }
            }
            PropertyAction::Toggle => {
                if property == atoms._NET_WM_STATE_MODAL {
                    window_props.is_modal = !window_props.is_modal;
                } else if property == atoms._NET_WM_STATE_STICKY {
                    window_props.is_sticky = !window_props.is_sticky;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_VERT {
                    window_props.is_maximized_vert = !window_props.is_maximized_vert;
                } else if property == atoms._NET_WM_STATE_MAXIMIZED_HORZ {
                    window_props.is_maximized_horz = !window_props.is_maximized_horz;
                } else if property == atoms._NET_WM_STATE_SHADED {
                    window_props.is_shaded = !window_props.is_shaded;
                } else if property == atoms._NET_WM_STATE_SKIP_TASKBAR {
                    window_props.skip_taskbar = !window_props.skip_taskbar;
                } else if property == atoms._NET_WM_STATE_SKIP_PAGER {
                    window_props.skip_pager = !window_props.skip_pager;
                } else if property == atoms._NET_WM_STATE_HIDDEN {
                    window_props.is_hidden = !window_props.is_hidden;
                } else if property == atoms._NET_WM_STATE_FULLSCREEN {
                    window_props.is_fullscreen = !window_props.is_fullscreen;
                } else if property == atoms._NET_WM_STATE_ABOVE {
                    window_props.above = !window_props.above;
                } else if property == atoms._NET_WM_STATE_BELOW {
                    window_props.below = !window_props.below;
                } else if property == atoms._NET_WM_STATE_DEMANDS_ATTENTION {
                    window_props.demands_attention = !window_props.demands_attention;
                }
            }
        }
    }
}

pub enum PropertyAction {
    Remove,
    Add,
    Toggle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowType {
    Desktop,
    Dialog,
    Dock,
    Menu,
    Normal,
    Splash,
    Toolbar,
    Utility,
}

#[derive(Default)]
pub struct BoundingBox {
    x: i16,
    pub y: i16,
    pub height: u16,
    width: u16,
}

impl BoundingBox {
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
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
    pub fn with_bbox(bounding_box: BoundingBox, bar_height: u16) -> Self {
        Self {
            output: String::new(),
            bar: Bar {
                window: None,
                show: true,
                pos: BarPosition::Top,
                x: 0,
                y: 0,
                status_text: String::new(),
                height: bar_height,
                width: bounding_box.width,
                _phantom_data: PhantomData,
            },
            bounding_box,
            root: Default::default(),
            _phantom_data: PhantomData,
        }
    }

    pub fn new(bar_width: u16, bar_height: u16) -> Self {
        Self {
            output: String::new(),
            bounding_box: BoundingBox::new(0, 0, 0, 0),
            root: Default::default(),
            bar: Bar {
                window: None,
                show: true,
                pos: BarPosition::Top,
                x: 0,
                y: 0,
                status_text: String::new(),
                height: bar_height,
                width: bar_width,
                _phantom_data: PhantomData,
            },
            _phantom_data: PhantomData,
        }
    }

    pub fn get_monitors(wm: &WM<'a, C>) -> Result<Vec<Monitor<'a, C>>, XlibError> {
        let root = wm.screen();
        let monitors = wm
            .conn_wrapper
            .connection
            .randr_get_monitors(root.root, true)?
            .reply()?
            .monitors;

        monitors
            .iter()
            .map(|m| {
                let output_info = wm
                    .conn_wrapper
                    .connection
                    .randr_get_output_info(
                        *m.outputs.first().expect("monitor output"),
                        CURRENT_TIME,
                    )?
                    .reply()?;

                let crtc = wm
                    .conn_wrapper
                    .connection
                    .randr_get_crtc_info(output_info.crtc, CURRENT_TIME)?
                    .reply()?;

                let mut monitor = Monitor::with_bbox(
                    BoundingBox::new(crtc.x, crtc.y, crtc.width, crtc.height),
                    30,
                );

                monitor.root = root.root;
                monitor.output = String::from_utf8(output_info.name).expect("output name utf8");

                Ok(monitor)
            })
            .collect()
    }
}

pub fn run<C: Connection>(mut wm: WM<'_, C>) -> Result<(), XlibError> {
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
        wm.conn_wrapper.connection.flush()?;

        let event = wm.conn_wrapper.connection.wait_for_event()?;
        let mut event_option = Some(event);
        while let Some(event) = event_option {
            // if let x11rb::protocol::Event::ClientMessage(_) = event {
            //     // This is start_timeout_thread() signaling us to close (most likely).
            //     return Ok(());
            // }

            wm.handle_event(event)?;
            event_option = wm.conn_wrapper.connection.poll_for_event()?;
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

    #[error("first property is missing")]
    PropertyMissing,

    #[error("invalid property action")]
    InvalidAction,
}
