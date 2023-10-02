use indexmap::IndexMap;

use crate::WindowState;

pub type WindowHandle = u32;

#[derive(Debug, Default)]
pub struct Windows {
    windows: IndexMap<WindowHandle, WindowState>,
    unmanaged_windows: Vec<WindowState>,
    focus: Option<WindowHandle>,
    previous_focus: Option<WindowHandle>,
}

impl Windows {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_window(&self, window_handle: WindowHandle) -> Option<&WindowState> {
        self.windows.get(&window_handle)
    }
    pub fn get_window_by<P>(&self, predicate: P) -> Option<(&u32, &WindowState)>
    where
        P: FnMut(&(&u32, &WindowState)) -> bool,
    {
        self.windows.iter().find(predicate)
    }
    pub fn get_window_mut_by<P>(&mut self, predicate: P) -> Option<(&u32, &mut WindowState)>
    where
        P: FnMut(&(&u32, &mut WindowState)) -> bool,
    {
        self.windows.iter_mut().find(predicate)
    }
    pub fn add_window(&mut self, window_handle: WindowHandle, win_state: WindowState) {
        self.windows.insert(window_handle, win_state);
    }
    pub fn add_unmanaged_window(&mut self, win_state: WindowState) {
        self.unmanaged_windows.push(win_state);
    }
    /// removes a window and shifts all windows that follow it
    /// and moves focus to the next index
    pub fn remove_window(
        &mut self,
        window_handle: WindowHandle,
    ) -> Option<(WindowHandle, WindowState)> {
        if let Some(window) = self.windows.shift_remove_entry(&window_handle) {
            self.previous_focus = self.focus;
            self.focus = self.next_window(window_handle).map(|(wh, _)| wh).copied();
            return Some(window);
        }

        None
    }
    /// gets the next window, and wraps around if the provided window is at `self.windows.len() - 1`
    pub fn next_window(
        &self,
        window_handle: WindowHandle,
    ) -> Option<(&WindowHandle, &WindowState)> {
        let Some(window_index) = self.windows.get_index_of(&window_handle) else {
            return None;
        };

        let next_window_index = (window_index + 1) % (self.windows.len() - 1);

        self.windows.get_index(next_window_index)
    }
    /// focused the provided window, and returns the previously focused window
    pub fn focus_window(
        &mut self,
        window_handle: WindowHandle,
    ) -> Result<Option<&WindowState>, WindowError> {
        self.windows
            .get(&window_handle)
            .ok_or(WindowError::InvalidWindowFocus)?;

        self.previous_focus = self.focus;
        self.focus = Some(window_handle);

        if let Some(previous_focus) = self.previous_focus {
            return Ok(self.windows.get(&previous_focus));
        }

        Ok(None)
    }
    /// removes current focus
    pub fn unfocus(&mut self) {
        self.previous_focus = self.focus;
        self.focus = None;
    }
    /// swaps the order of 2 windows.
    /// does nothing if one or more doesn't exist
    pub fn swap_windows(&mut self, first_window: WindowHandle, second_window: WindowHandle) {
        let Some(first) = self.windows.get_index_of(&first_window) else {
            return;
        };
        let Some(second) = self.windows.get_index_of(&second_window) else {
            return;
        };
        self.windows.swap_indices(first, second);
    }
    /// moves the provided window to the top of the stack
    /// does nothing if provided window doesn't exist in the stack
    pub fn move_to_top(&mut self, window_handle: WindowHandle) {
        let Some(window_index) = self.windows.get_index_of(&window_handle) else {
            return;
        };
        self.windows.move_index(window_index, 0);
    }
    pub fn focused(&self) -> Option<&WindowState> {
        let Some(focus) = self.focus else {
            return None;
        };

        self.windows.get(&focus)
    }
    pub fn focused_mut(&mut self) -> Option<&mut WindowState> {
        let Some(focus) = self.focus else {
            return None;
        };

        self.windows.get_mut(&focus)
    }
    pub fn previos_focus(&self) -> Option<&WindowState> {
        let Some(previos_focus) = self.previous_focus else {
            return None;
        };

        self.windows.get(&previos_focus)
    }
    pub fn windows(&self) -> Vec<&WindowState> {
        self.windows.values().collect()
    }
    pub fn windows_mut(&mut self) -> Vec<&mut WindowState> {
        self.windows.values_mut().collect()
    }
    pub fn unmanaged_windows(&self) -> Vec<&WindowState> {
        self.unmanaged_windows.iter().collect()
    }
    pub fn umanaged_windows_mut(&mut self) -> Vec<&mut WindowState> {
        self.unmanaged_windows.iter_mut().collect()
    }
    pub fn floating_windows(&self) -> Vec<&WindowState> {
        self.windows()
            .into_iter()
            .filter(|w| w.is_floating)
            .collect()
    }
    pub fn floating_windows_handles(&self) -> Vec<&u32> {
        self.windows
            .iter()
            .filter_map(|(wh, w)| if w.is_floating { Some(wh) } else { None })
            .collect()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WindowError {
    #[error("provided window can't be focused")]
    InvalidWindowFocus,
}
