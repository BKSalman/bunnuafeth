use bunnuafeth::{run, wm::WM, Config, Hotkey, MouseHotkey, WMCommand};
use x11rb::{
    connect,
    protocol::xproto::{ButtonIndex, ModMask},
};

fn init_tracing() {
    use tracing_subscriber::prelude::*;

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

fn main() {
    init_tracing();
    tracing::info!("Connecting to x11 server");
    let (connection, screen_num) = connect(None).expect("connect to X11 server");

    let conn1 = std::sync::Arc::new(connection);
    let conn = &*conn1;

    let hotkeys = vec![
        Hotkey::new(
            ModMask::M1,
            x11_keysyms::XK_q,
            WMCommand::Execute(String::from("kitty")),
        ),
        Hotkey::new(
            ModMask::M1,
            x11_keysyms::XK_p,
            WMCommand::Execute(String::from("xprop")),
        ),
        Hotkey::new(ModMask::M1, x11_keysyms::XK_c, WMCommand::CloseWindow),
        Hotkey::new(ModMask::M1, x11_keysyms::XK_Alt_L, WMCommand::MoveWindow),
    ];

    let mouse_hotkeys = vec![MouseHotkey::new(
        ModMask::M1,
        ButtonIndex::M1,
        WMCommand::MoveWindow,
    )];

    let config = Config {
        hotkeys,
        mouse_hotkeys,
    };

    let mut wm = WM::new(conn, screen_num, config).expect("create drawable");
    // 6275a6
    //                                red  green  blue
    wm.set_root_background_color(0x00__62__75_____a6__).unwrap();

    wm.create_bar().unwrap();
    wm.bar.update_position(&wm).unwrap();

    wm.scan_windows().expect("scan windows");

    // Monitor::get_monitors(&wm)
    //     .expect("get monitors")
    //     .iter_mut()
    //     .for_each(|monitor| {
    //         ;
    //         monitor
    //             .bar
    //             .update_position(&wm)
    //             .expect("update bar position");
    //     });

    wm.setup().expect("setup window manager");

    run(wm).expect("run window manager");
}
