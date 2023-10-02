use crate::some_if_changed;
use x11rb::protocol::xproto::Window;

use crate::{wm::BORDER_WIDTH, WindowProperties, WindowState, WindowType};

pub enum Layout {
    Floating,
    Tiled(TiledLayout),
}

pub enum TiledLayout {
    MainStack,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EdgeDimensions {
    pub width: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReservedEdges {
    pub top: EdgeDimensions,
    pub right: EdgeDimensions,
    pub left: EdgeDimensions,
    pub bottom: EdgeDimensions,
}

pub struct LayoutManager {
    pub layout: Layout,
    pub reserved: ReservedEdges,
}

#[derive(Default, Debug)]
pub struct WindowStateDiff {
    pub x: Option<i16>,
    pub y: Option<i16>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub window: Window,
    pub r#type: Option<WindowType>,
    pub properties: Option<WindowProperties>,
    pub is_floating: Option<bool>,
}

impl LayoutManager {
    pub fn calculate_dimensions(
        &self,
        windows: Vec<&WindowState>,
        screen_width: u16,
        screen_height: u16,
    ) -> Option<Vec<WindowStateDiff>> {
        match &self.layout {
            Layout::Floating => None,
            Layout::Tiled(tiled_layout) => match tiled_layout {
                TiledLayout::MainStack => {
                    let mut windows: Vec<WindowState> = windows
                        .into_iter()
                        .cloned()
                        .filter(|w| w.r#type == WindowType::Normal && !w.is_floating)
                        .collect();
                    let mut windows_final: Vec<WindowStateDiff> = Vec::new();
                    let screen_height = screen_height
                        - self.reserved.top.width as u16
                        - self.reserved.bottom.width as u16;

                    if let Some((main, windows)) = windows.split_first_mut() {
                        let sub_windows_count = windows.len();
                        let mut main_diff = WindowStateDiff {
                            window: main.window,
                            ..Default::default()
                        };

                        main_diff.height = some_if_changed!(
                            main.height,
                            screen_height - (BORDER_WIDTH * 2) as u16
                        );

                        main_diff.width = some_if_changed!(
                            main.width,
                            screen_width
                                - (BORDER_WIDTH * 2) as u16
                                - self.reserved.right.width as u16
                                - self.reserved.left.width as u16
                        );

                        // FIXME: deal with the insane amount of casts

                        main_diff.y = some_if_changed!(main.y, self.reserved.top.width as i16);
                        main_diff.x = some_if_changed!(main.x, self.reserved.left.width as i16);

                        let mut sub_windows = Vec::new();

                        if sub_windows_count >= 1 {
                            main_diff.width = some_if_changed!(
                                main.width,
                                (screen_width / 2) - (BORDER_WIDTH * 2) as u16
                            );

                            sub_windows = windows
                                .iter_mut()
                                .enumerate()
                                .map(|(i, win_state)| {
                                    let height = screen_height / sub_windows_count as u16
                                        - (BORDER_WIDTH * 2) as u16;

                                    WindowStateDiff {
                                        x: some_if_changed!(win_state.x, (screen_width / 2) as i16),
                                        y: some_if_changed!(
                                            win_state.y,
                                            (((height + (BORDER_WIDTH * 2) as u16) * i as u16)
                                                + (self.reserved.top.width as u16))
                                                as i16
                                        ),
                                        width: some_if_changed!(
                                            win_state.width,
                                            (screen_width / 2) - (BORDER_WIDTH * 2) as u16
                                        ),
                                        height: some_if_changed!(win_state.height, height),
                                        window: win_state.window,
                                        ..Default::default()
                                    }
                                })
                                .collect();
                        }
                        windows_final.push(main_diff);
                        windows_final.extend(sub_windows);

                        return Some(windows_final);
                    }
                    return None;
                }
            },
        }
    }
}

#[macro_export]
macro_rules! some_if_changed {
    ($old:expr, $new:expr) => {
        if $old != $new {
            Some($new)
        } else {
            None
        }
    };
}
