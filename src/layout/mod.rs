use crate::{wm::BORDER_WIDTH, WindowState, WindowType};

pub enum Layout {
    Floating,
    Tiled(TiledLayout),
}

pub enum TiledLayout {
    MainStack,
}

#[derive(Default)]
pub struct EdgeDimensions {
    pub width: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Default)]
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

impl LayoutManager {
    pub fn calculate_dimensions(
        &self,
        windows: Vec<WindowState>,
        screen_width: u16,
        screen_height: u16,
    ) -> Option<Vec<WindowState>> {
        match &self.layout {
            Layout::Floating => None,
            Layout::Tiled(tiled_layout) => match tiled_layout {
                TiledLayout::MainStack => {
                    let mut windows: Vec<WindowState> = windows
                        .into_iter()
                        .filter(|w| w.r#type == WindowType::Normal)
                        .collect();
                    let mut windows_final: Vec<WindowState> = Vec::new();
                    let screen_height = screen_height - self.reserved.top.width as u16;

                    if let Some((main, windows)) = windows.split_first_mut() {
                        let sub_windows_count = windows.len();

                        main.height = screen_height
                            - BORDER_WIDTH as u16
                            - self.reserved.top.width as u16
                            - self.reserved.bottom.width as u16;

                        main.width = screen_width
                            - (BORDER_WIDTH * 2) as u16
                            - self.reserved.right.width as u16
                            - self.reserved.left.width as u16;

                        // FIXME: deal with the insane amount of casts

                        main.y = self.reserved.top.width as i16;
                        main.x = self.reserved.left.width as i16;
                        let mut sub_windows = Vec::new();
                        if sub_windows_count >= 1 {
                            main.width = (screen_width / 2) - (BORDER_WIDTH * 2) as u16;

                            sub_windows = windows
                                .iter_mut()
                                .enumerate()
                                .map(|(i, window)| {
                                    window.width = (screen_width / 2) - (BORDER_WIDTH * 2) as u16;
                                    window.x = (screen_width / 2) as i16;
                                    window.height = screen_height / sub_windows_count as u16
                                        - (BORDER_WIDTH * 2) as u16;
                                    window.y = ((window.height + (BORDER_WIDTH * 2) as u16)
                                        * i as u16)
                                        as i16;
                                    window.clone()
                                })
                                .collect();
                        }
                        windows_final.push(main.clone());
                        windows_final.extend(sub_windows);

                        return Some(windows_final);
                    }
                    return None;
                }
            },
        }
    }
}
