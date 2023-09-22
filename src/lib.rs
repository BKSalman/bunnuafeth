use bar::{Bar, BarPosition};
use std::{
    io::{BufRead, BufReader},
    marker::PhantomData,
};
use wm::WM;
use x11rb::protocol::{randr::ConnectionExt, xproto::ButtonIndex};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConnectionExt as XlibConnectionExt, CreateWindowAux, GetGeometryReply, ModMask, Window,
        WindowClass,
    },
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, CURRENT_TIME,
};

mod atoms;
mod bar;
mod util;
pub mod wm;

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

#[derive(Debug, Clone)]
pub enum WMCommand {
    Execute(String),
    CloseWindow,
    MoveWindow,
    /// the parameter here is not needed for mouse resizing
    ResizeWindow(i16),
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

#[derive(Debug, Clone)]
pub struct WindowState {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    pub window: Window,
    is_bar: bool,
}

impl WindowState {
    fn new(window: Window, geom: &GetGeometryReply, is_bar: bool) -> WindowState {
        WindowState {
            window,
            x: geom.x,
            y: geom.y,
            width: geom.width,
            height: geom.height,
            is_bar,
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
    pub root: Window,
    pub output: String,
    pub bounding_box: BoundingBox,
    pub bar: Bar<'a, C>,
    _phantom_data: PhantomData<&'a C>,
}

impl<'a, C: Connection> Monitor<'a, C> {
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
