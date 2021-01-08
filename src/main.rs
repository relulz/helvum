mod pipewire_connection;
mod pipewire_state;
mod view;

use gtk::prelude::*;

use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct PipewireLink {
    pub node_from: u32,
    pub port_from: u32,
    pub node_to: u32,
    pub port_to: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let graphview = Rc::new(RefCell::new(view::GraphView::new()));

    // Create the connection to the pipewire server and do an initial roundtrip before showing the view,
    // so that the graph is already populated when the window opens.
    let pw_con = pipewire_connection::PipewireConnection::new(pipewire_state::PipewireState::new(
        graphview.clone(),
    ))
    .expect("Failed to initialize pipewire connection");
    pw_con.roundtrip();
    // From now on, call roundtrip() every second.
    gtk::glib::timeout_add_seconds_local(1, move || {
        pw_con.roundtrip();
        Continue(true)
    });

    let app = gtk::Application::new(Some("org.freedesktop.pipewire.graphui"), Default::default())
        .expect("Application creation failed");

    app.connect_activate(move |app| {
        let scrollwindow = gtk::ScrolledWindowBuilder::new()
            .child(&*graphview.borrow())
            .build();
        let window = gtk::ApplicationWindowBuilder::new()
            .application(app)
            .default_width(1280)
            .default_height(720)
            .title("Pipewire Graph Editor")
            .child(&scrollwindow)
            .build();
        window
            .get_settings()
            .set_property_gtk_application_prefer_dark_theme(true);
        window.show();
    });

    app.run(&std::env::args().collect::<Vec<_>>());

    Ok(())
}
