use std::marker::PhantomData;

use x11rb::{connection::Connection, protocol::xproto::Window};

use crate::{wm::WM, Monitor, XlibError};

pub struct Bar<'a, C: Connection> {
    pub window: Option<Window>,
    pub show: bool,
    pub pos: BarPosition,
    pub y: i32,
    pub status_text: String,
    pub height: i32,
    pub _phantom_data: PhantomData<&'a C>,
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
