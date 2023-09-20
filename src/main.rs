use bunnuafeth::{run, setup_wm_attrs, Monitor, WM};
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
    tracing::info!("Creating Drawable");
    let (connection, screen_num) = connect(None).expect("connect to X11 server");

    let conn1 = std::sync::Arc::new(connection);
    let conn = &*conn1;

    let drawable = WM::new(conn, screen_num).expect("create drawable");

    tracing::info!("Loading fonts");

    // drawable.load_font(&xft, "monospace");

    tracing::info!("Updating geometry");

    tracing::info!("Setting up root window and dummy window");

    Monitor::get_monitors(&drawable)
        .expect("get monitors")
        .iter_mut()
        .for_each(|monitor| {
            monitor.update_bar(&drawable).expect("update bar");
            monitor
                .bar
                .update_position(&drawable)
                .expect("update bar position");
        });

    setup_wm_attrs(&drawable).expect("setup window manager");

    run(&drawable.connection).expect("run window manager");
}
