use bunnuafeth::{run, setup_wm_attrs, WM};
use x11rb::connect;

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

    let mut wm = WM::new(conn, screen_num).expect("create drawable");

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

    setup_wm_attrs(&wm).expect("setup window manager");

    run(wm).expect("run window manager");
}
