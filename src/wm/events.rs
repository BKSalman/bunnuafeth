use crate::{ButtonMapping, WindowState, WindowType};
use std::{cmp::Reverse, process::Command};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            AtomEnum, ButtonPressEvent, ButtonReleaseEvent, ChangeWindowAttributesAux,
            ClientMessageEvent, ConfigureRequestEvent, ConfigureWindowAux, ConnectionExt,
            DestroyNotifyEvent, EnterNotifyEvent, ExposeEvent, KeyPressEvent, MapRequestEvent,
            MotionNotifyEvent, PropMode, SetMode, UnmapNotifyEvent,
        },
        Event,
    },
    rust_connection::ReplyError,
    wrapper::ConnectionExt as WrapperConnectionExt,
    CURRENT_TIME,
};

use crate::{util::CommandExt, KeyMapping, WMCommand, XlibError};

use super::{FocusWindow, WM};

impl<'a, C: Connection> WM<'a, C> {
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

        if matches!(event, Event::ConfigureNotify(_)) {
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
            Event::ClientMessage(event) => self.handle_client_message(event)?,
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
        self.conn_wrapper
            .connection
            .configure_window(event.window, &aux)?;
        Ok(())
    }

    fn handle_expose(&mut self, event: ExposeEvent) {
        self.pending_expose.insert(event.window);
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> Result<(), XlibError> {
        self.manage_window(
            event.window,
            &self
                .conn_wrapper
                .connection
                .get_geometry(event.window)?
                .reply()?,
        )
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), XlibError> {
        let button_mapping = ButtonMapping::new(event.detail, u16::from(event.state));

        if let Some(command) = self.button_mapping.get(&button_mapping) {
            match command {
                WMCommand::Execute(_) => todo!(),
                WMCommand::CloseWindow => todo!(),
                WMCommand::MoveWindow => {
                    if let Some(win_state) =
                        self.windows.iter_mut().find(|w| w.window == event.child)
                    {
                        win_state.is_floating = true;
                        let win_state = win_state.clone();

                        if !self.can_move(win_state.window) {
                            return Ok(());
                        }

                        self.conditionally_grab_pointer(win_state.window)?;

                        let change = ChangeWindowAttributesAux::new().cursor(self.cursors.r#move);
                        self.conn_wrapper
                            .connection
                            .change_window_attributes(win_state.window, &change)?;
                        let geometry = self
                            .conn_wrapper
                            .connection
                            .get_geometry(win_state.window)?
                            .reply()?;
                        self.drag_window = Some((
                            win_state.window,
                            (geometry.x - event.event_x, geometry.y - event.event_y),
                        ));
                        self.raise_window(win_state.window)?;
                        self.focus_window(FocusWindow::Normal(Some(&win_state)))?;
                    }

                    let screen = self.screen();
                    if let Some(new_windows) = self.layout_manager.calculate_dimensions(
                        self.windows.clone(),
                        screen.width_in_pixels,
                        screen.height_in_pixels,
                    ) {
                        self.apply_layout_diff(new_windows)?;
                    }
                }
                WMCommand::ResizeWindow(_) => {
                    if let Some(win_state) = self.windows.iter().find(|w| w.window == event.child) {
                        if !self.can_resize(win_state.window) {
                            return Ok(());
                        }

                        let window = win_state.window;
                        self.conditionally_grab_pointer(window)?;
                        let change = ChangeWindowAttributesAux::new().cursor(self.cursors.resize);
                        self.conn_wrapper
                            .connection
                            .change_window_attributes(window, &change)?;

                        let geometry =
                            self.conn_wrapper.connection.get_geometry(window)?.reply()?;
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
                WMCommand::ToggleFullscreen => {
                    if let Some(win_state) = &self.focused_window {
                        if win_state.properties.is_fullscreen {
                            self.unfullscreen_window(win_state.window)?;
                        } else {
                            self.fullscreen_window(win_state.window)?;
                        }
                    }
                }
                WMCommand::ToggleFloating => todo!(),
            }
        }
        Ok(())
    }

    fn handle_button_release(&mut self, _event: ButtonReleaseEvent) -> Result<(), XlibError> {
        if let Some(drag_window) = self.drag_window {
            let change = ChangeWindowAttributesAux::new().cursor(self.cursors.normal);
            self.conn_wrapper
                .connection
                .change_window_attributes(drag_window.0, &change)?;
        }
        if let Some(resize_window) = self.resize_window {
            let change = ChangeWindowAttributesAux::new().cursor(self.cursors.normal);
            self.conn_wrapper
                .connection
                .change_window_attributes(resize_window.0, &change)?;
        }

        self.drag_window = None;
        self.resize_window = None;
        self.pointer_grabbed = false;
        self.conn_wrapper.connection.ungrab_pointer(CURRENT_TIME)?;
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
                        .and_then(|fw| self.windows.iter().find(|w| w.window == fw.window))
                    {
                        if state.window == self.screen().root {
                            return Ok(());
                        }

                        self.send_delete(state.window)?;
                    }
                }
                WMCommand::MoveWindow => {
                    if let Some(win_state) = self.windows.iter().find(|w| w.window == event.event) {
                        if !self.can_resize(win_state.window) {
                            return Ok(());
                        }

                        let change = ChangeWindowAttributesAux::new().cursor(self.cursors.r#move);
                        self.conn_wrapper
                            .connection
                            .change_window_attributes(event.event, &change)?;

                        if self.drag_window.is_none() {
                            let (x, y) = (-event.event_x, -event.event_y);
                            self.drag_window = Some((win_state.window, (x, y)));
                        }
                    }
                }
                WMCommand::ResizeWindow(_factor) => todo!(),
                WMCommand::ToggleFullscreen => {
                    if let Some(fw_state) = &self.focused_window {
                        if fw_state.properties.is_fullscreen {
                            self.unfullscreen_window(fw_state.window)?;
                        } else {
                            self.fullscreen_window(fw_state.window)?;
                        }
                    }
                }
                WMCommand::ToggleFloating => {
                    if let Some(fw_state) = &self.focused_window {
                        if let Some(win_state) = self
                            .windows
                            .iter_mut()
                            .find(|w| w.window == fw_state.window)
                        {
                            win_state.is_floating = !win_state.is_floating;
                            let screen = self.screen();
                            if let Some(new_windows) = self.layout_manager.calculate_dimensions(
                                self.windows.clone(),
                                screen.width_in_pixels,
                                screen.height_in_pixels,
                            ) {
                                self.apply_layout_diff(new_windows)?;
                            }
                        }
                    }
                }
            };
        }

        Ok(())
    }

    fn handle_unmap_notify(&mut self, event: UnmapNotifyEvent) -> Result<(), XlibError> {
        if self
            .windows
            .iter()
            .find(|w| w.window == event.window)
            .is_some()
        {
            let root = self.screen().root;
            self.focus_window(FocusWindow::Root(root))?;
            self.windows.retain(|state| {
                if state.window != event.window {
                    return true;
                }
                self.conn_wrapper
                    .connection
                    .change_save_set(SetMode::DELETE, state.window)
                    .unwrap();
                self.conn_wrapper
                    .connection
                    .reparent_window(state.window, root, state.x, state.y)
                    .unwrap();
                false
            });

            let screen = self.screen();

            if let Some(new_windows) = self.layout_manager.calculate_dimensions(
                self.windows.clone(),
                screen.width_in_pixels,
                screen.height_in_pixels,
            ) {
                self.apply_layout_diff(new_windows)?;
            }
        }

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
            self.conn_wrapper
                .connection
                .configure_window(win, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
            if let Some(state) = self.windows.iter_mut().find(|w| w.window == win) {
                state.x = x;
                state.y = y;
            }
        } else if let Some((win, ((width, height), (x, y)))) = self.resize_window {
            let (width, height) = (
                width as i16 + x + event.event_x,
                height as i16 + y + event.event_y,
            );
            self.conn_wrapper.connection.configure_window(
                win,
                &ConfigureWindowAux::new()
                    .width(width as u32)
                    .height(height as u32),
            )?;
            if let Some(state) = self.windows.iter_mut().find(|w| w.window == win) {
                state.width = width as u16;
                state.height = height as u16;
            }
        }
        Ok(())
    }

    fn handle_enter(&mut self, event: EnterNotifyEvent) -> Result<(), XlibError> {
        // TODO: can I remove this clone?
        let win = self
            .windows
            .iter()
            .find(|w| w.window == event.event && w.r#type == WindowType::Normal)
            .cloned();
        // tracing::debug!("focusing {win:?}");
        self.focus_window(FocusWindow::Normal(win.as_ref()))?;
        // TODO: add border when focusing window
        // let change = ChangeWindowAttributesAux::new().border_pixel(self.black_gc);
        // self.conn_wrapper.connection.change_window_attributes(event.event)
        Ok(())
    }

    fn handle_leave(&mut self, event: EnterNotifyEvent) -> Result<(), XlibError> {
        if let Some((win, focused_window)) = self
            .windows
            .iter()
            .find(|w| w.window == event.event)
            .zip(self.focused_window.as_ref())
        {
            if focused_window.window == win.window {
                // tracing::debug!("unfocusing {win:?} and focusing root window");
                let root = self.screen().root;
                self.focus_window(FocusWindow::Root(root)).unwrap();
            }
        }

        Ok(())
    }

    fn handle_destroy_notify(&mut self, event: DestroyNotifyEvent) -> Result<(), XlibError> {
        if self
            .windows
            .iter()
            .find(|w| w.window == event.window)
            .is_some()
        {
            let root = self.screen().root;
            self.windows.retain(|state| {
                if state.window != event.window {
                    return true;
                }
                self.conn_wrapper
                    .connection
                    .change_save_set(SetMode::DELETE, state.window)
                    .unwrap();
                self.conn_wrapper
                    .connection
                    .reparent_window(state.window, root, state.x, state.y)
                    .unwrap();
                // self.conn_wrapper.connection.destroy_window(state.frame_window).unwrap();
                false
            });

            let managed: Vec<_> = self.windows.iter().map(|w| w.window).collect();

            self.conn_wrapper.connection.change_property32(
                PropMode::REPLACE,
                self.screen().root,
                self.conn_wrapper.atoms._NET_CLIENT_LIST,
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
            let screen = self.screen();

            if let Some(new_windows) = self.layout_manager.calculate_dimensions(
                self.windows.clone(),
                screen.width_in_pixels,
                screen.height_in_pixels,
            ) {
                self.apply_layout_diff(new_windows)?;
            }
        }

        Ok(())
    }

    fn handle_client_message(&mut self, event: ClientMessageEvent) -> Result<(), XlibError> {
        if event.type_ == self.conn_wrapper.atoms._NET_WM_STATE {
            let data = event.data.as_data32();

            // https://specifications.freedesktop.org/wm-spec/1.3/ar01s05.html#idm45798289450576

            // 0 = remove, 1 = add, 2 = toggle
            let action = data[0];
            // This message allows two properties to be changed simultaneously,
            // specifically to allow both horizontal and vertical maximization to be altered together
            let first_property = data[1];
            let second_property = data[2];

            let atoms = self.conn_wrapper.atoms;

            if let Some(win_state) = self.windows.iter_mut().find(|w| w.window == event.window) {
                let action = WindowState::get_property_action(action)?;
                WindowState::set_window_property(
                    atoms,
                    first_property,
                    &action,
                    &mut win_state.properties,
                );
                WindowState::set_window_property(
                    atoms,
                    second_property,
                    &action,
                    &mut win_state.properties,
                );
                tracing::debug!("new window state: {:?}", win_state);
            }

            // whether the source is an application or direct user actions
            // TODO: I don't know what to do with it yet
            // let source_indication = data[3];
        }

        Ok(())
    }
}
