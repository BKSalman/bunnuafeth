use std::marker::PhantomData;

use x11rb::{
    connection::Connection,
    protocol::xproto::{
        AtomEnum, ConnectionExt, CreateWindowAux, EventMask, PropMode, Rectangle, Window,
        WindowClass,
    },
    wrapper::ConnectionExt as WrapperConnectionExt,
    COPY_DEPTH_FROM_PARENT,
};

use crate::{wm::WM, Monitor, WindowState, WindowType, XlibError};

pub struct Bar<'a, C: Connection> {
    pub window: Option<Window>,
    pub show: bool,
    pub pos: BarPosition,
    pub y: i16,
    pub x: i16,
    pub status_text: String,
    pub height: u16,
    pub width: u16,
    pub _phantom_data: PhantomData<&'a C>,
}

impl<'a, C: Connection> Bar<'a, C> {
    pub fn update_position(&self, wm: &WM<'a, C>) -> Result<(), XlibError> {
        // let mut monitors = Monitor::get_monitors(drawable)?;

        // for monitor in monitors.iter_mut() {
        //     if monitor.bar.show {
        //         monitor.bounding_box.height -= self.height;
        //         match monitor.bar.pos {
        //             BarPosition::Top => {
        //                 monitor.bar.y = monitor.bounding_box.y;
        //                 monitor.bounding_box.y += self.height;
        //             }
        //             BarPosition::Bottom => {
        //                 monitor.bar.y = monitor.bounding_box.height + monitor.bounding_box.y;
        //             }
        //         }
        //     } else {
        //         monitor.bar.y = -self.height;
        //     }

        // if self.show {
        //     monitor.bounding_box.height -= self.height;
        //     match monitor.bar.pos {
        //         BarPosition::Top => {
        //             monitor.bar.y = monitor.bounding_box.y;
        //             monitor.bounding_box.y += self.height;
        //         }
        //         BarPosition::Bottom => {
        //             monitor.bar.y = monitor.bounding_box.height + monitor.bounding_box.y;
        //         }
        //     }
        // } else {
        //     monitor.bar.y = -self.height;
        // }

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

impl<'a, C: Connection> WM<'a, C> {
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
}
