use x11rb::protocol::xproto::Atom;

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
        _NET_WM_WINDOW_TYPE_NORMAL,

        _NET_CLIENT_LIST,
        _NET_DESKTOP_VIEWPORT,
        _NET_DESKTOP_GEOMETRY,
        _NET_NUMBER_OF_DESKTOPS,
        _NET_CURRENT_DESKTOP,
        _NET_DESKTOP_NAMES,
        _NET_WORKAREA,
        _NET_WM_DESKTOP,
        _NET_WM_STRUT,
        _NET_FRAME_EXTENTS,

        _NET_WM_NAME,

        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
    }
}

impl Atoms {
    pub fn net_supported(&self) -> Vec<Atom> {
        vec![
            // STATE
            self._NET_WM_STATE,
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
            // ACTION
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
            // WINDOW
            self._NET_WM_WINDOW_TYPE,
            self._NET_WM_WINDOW_TYPE_DESKTOP,
            self._NET_WM_WINDOW_TYPE_DOCK,
            self._NET_WM_WINDOW_TYPE_TOOLBAR,
            self._NET_WM_WINDOW_TYPE_MENU,
            self._NET_WM_WINDOW_TYPE_UTILITY,
            self._NET_WM_WINDOW_TYPE_SPLASH,
            self._NET_WM_WINDOW_TYPE_DIALOG,
            // other
            self._NET_SUPPORTING_WM_CHECK,
            self._NET_DESKTOP_VIEWPORT,
            self._NET_NUMBER_OF_DESKTOPS,
            self._NET_CURRENT_DESKTOP,
            self._NET_DESKTOP_NAMES,
            self._NET_WM_DESKTOP,
            self._NET_WM_DESKTOP,
            self._NET_WM_STRUT,
            self._NET_CLIENT_LIST,
            self._NET_ACTIVE_WINDOW,
            self._NET_DESKTOP_GEOMETRY,
            self._NET_SUPPORTED,
            self._NET_WM_NAME,
            self._NET_WM_ALLOWED_ACTIONS,
            self._NET_WM_PID,
        ]
    }
}