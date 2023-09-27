use crate::{WindowState, WindowType};

pub enum Layout {
    Floating,
    Tiled(TiledLayout),
}

pub enum TiledLayout {
    MainStack,
}

pub struct LayoutManager {
    pub layout: Layout,
}

impl LayoutManager {
    fn calculate_dimensions(
        &self,
        windows: &[WindowState],
        screen_size: (u16, u16),
    ) -> Option<Vec<WindowState>> {
        match &self.layout {
            Layout::Floating => None,
            Layout::Tiled(tiled_layout) => match tiled_layout {
                TiledLayout::MainStack => {
                    let mut windows = windows.to_vec();
                    let windows_count = windows.len();
                    let mut windows = windows
                        .iter_mut()
                        .filter(|w| w.r#type != WindowType::Normal);

                    let mut windows_final: Vec<WindowState> = Vec::new();

                    if let Some(main) = windows.next() {
                        main.width = screen_size.0 / 2;
                        let windows_height =
                            screen_size.1 / TryInto::<u16>::try_into(windows_count).unwrap();

                        windows_final.push(main.clone());

                        for window in windows {
                            window.width = screen_size.0 / 2;
                            window.height = windows_height;
                            windows_final.push(window.clone());
                        }

                        return Some(windows_final);
                    }
                    return None;
                }
            },
        }
    }
}
