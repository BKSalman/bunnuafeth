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

use crate::{wm::WM, WindowState, WindowType, XlibError, BAR_HEIGHT, RGBA};

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
        let bar_win_id = self.conn_wrapper.connection.generate_id()?;

        let root = &self.conn_wrapper.connection.setup().roots[self.screen_num];

        let window_aux = CreateWindowAux::new()
            .event_mask(EventMask::BUTTON_PRESS | EventMask::EXPOSURE)
            .override_redirect(Some(true.into()))
            .background_pixel(RGBA::new(0xff, 0xff, 0xff, 0).as_argb_u32());

        self.conn_wrapper.connection.create_window(
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

        self.conn_wrapper.connection.change_property8(
            PropMode::REPLACE,
            bar_win_id,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            "Bunnuafeth bar".as_bytes(),
        )?;
        self.conn_wrapper.connection.change_property8(
            PropMode::REPLACE,
            bar_win_id,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            "bunnuafeth-bar".as_bytes(),
        )?;

        self.conn_wrapper.connection.change_property32(
            PropMode::REPLACE,
            bar_win_id,
            self.conn_wrapper.atoms._NET_WM_WINDOW_TYPE,
            AtomEnum::ATOM,
            &[self.conn_wrapper.atoms._NET_WM_WINDOW_TYPE_DOCK],
        )?;

        self.conn_wrapper.connection.change_property32(
            PropMode::REPLACE,
            bar_win_id,
            self.conn_wrapper.atoms._NET_WM_STRUT_PARTIAL,
            AtomEnum::CARDINAL,
            &[
                // left
                0,
                // right
                0,
                // top
                BAR_HEIGHT as u32,
                // bottom
                0,
                // left_start_y
                0,
                // left_end_y
                0,
                // right_start_y
                0,
                // right_end_y
                0,
                // top_start_x
                0,
                // top_end_x
                self.screen().width_in_pixels as u32,
                // bottom_start_x
                0,
                // bottom_end_x
                0,
            ],
        )?;

        let geom = self
            .conn_wrapper
            .connection
            .get_geometry(bar_win_id)?
            .reply()?;

        self.manage_window(bar_win_id, &geom)?;

        Ok(())
    }

    pub fn draw_bar(&self) -> Result<(), XlibError> {
        if let Some(bar_window) = self.bar.window {
            // self.conn_wrapper
            //     .connection
            //     .poly_fill_rectangle(
            //         bar_window,
            //         self.black_gc,
            //         &[Rectangle {
            //             x: self.bar.x,
            //             y: self.bar.y,
            //             width: self.bar.width,
            //             height: self.bar.height,
            //         }],
            //     )?
            //     .check()?;
            if let Some(fw_state) = &self.focused_window {
                let reply = self
                    .conn_wrapper
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
                self.conn_wrapper
                    .connection
                    .image_text8(
                        bar_window,
                        self.black_gc,
                        1,
                        10,
                        fw_state.window.to_string().as_bytes(),
                    )?
                    .check()?;
            } else {
                self.conn_wrapper.connection.image_text8(
                    bar_window,
                    self.black_gc,
                    1,
                    10,
                    b"something",
                )?;
            }
        }

        Ok(())
    }
}
