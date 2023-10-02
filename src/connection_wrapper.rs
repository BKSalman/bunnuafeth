use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, PropMode, Window},
    wrapper::ConnectionExt,
};

use crate::{atoms::Atoms, WindowProperties, XlibError};

pub struct ConnWrapper<'a, C: Connection> {
    pub connection: &'a C,
    pub root: u32,
    pub atoms: Atoms,
}

impl<'a, C: Connection> ConnWrapper<'a, C> {
    pub fn update_net_wm_state(
        &self,
        window_props: &WindowProperties,
        window: Window,
    ) -> Result<(), XlibError> {
        let mut props = Vec::new();

        if window_props.is_fullscreen {
            props.push(self.atoms._NET_WM_STATE_FULLSCREEN);
        }
        if window_props.is_sticky {
            props.push(self.atoms._NET_WM_STATE_STICKY);
        }
        if window_props.is_modal {
            props.push(self.atoms._NET_WM_STATE_MODAL);
        }
        if window_props.is_maximized_horz {
            props.push(self.atoms._NET_WM_STATE_MAXIMIZED_HORZ);
        }
        if window_props.is_maximized_vert {
            props.push(self.atoms._NET_WM_STATE_MAXIMIZED_VERT);
        }
        if window_props.is_shaded {
            props.push(self.atoms._NET_WM_STATE_SHADED);
        }
        if window_props.skip_taskbar {
            props.push(self.atoms._NET_WM_STATE_SKIP_TASKBAR);
        }
        if window_props.skip_pager {
            props.push(self.atoms._NET_WM_STATE_SKIP_PAGER);
        }
        if window_props.is_hidden {
            props.push(self.atoms._NET_WM_STATE_HIDDEN);
        }
        if window_props.above {
            props.push(self.atoms._NET_WM_STATE_ABOVE);
        }
        if window_props.below {
            props.push(self.atoms._NET_WM_STATE_BELOW);
        }
        if window_props.demands_attention {
            props.push(self.atoms._NET_WM_STATE_DEMANDS_ATTENTION);
        }

        self.connection.change_property32(
            PropMode::REPLACE,
            window,
            self.atoms._NET_WM_STATE,
            AtomEnum::ATOM,
            props.as_slice(),
        )?;

        Ok(())
    }
}
