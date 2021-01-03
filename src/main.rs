mod pipewire_connection;
mod view;

use gio::prelude::*;
use glib::clone;
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
    let pw_con = pipewire_connection::PipewireConnection::new(graphview.clone())
        .expect("Failed to initialize pipewire connection");
    pw_con.roundtrip();
    // From now on, call roundtrip() every second.
    glib::timeout_add_seconds_local(1, move || {
        pw_con.roundtrip();
        Continue(true)
    });

    let app = gtk::Application::new(
        Some("org.freedesktop.pipewire.graphui"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .expect("Application creation failed");

    app.connect_activate(clone!(@strong graphview => move |app| {
        let window = gtk::ApplicationWindow::new(app);
        window.set_default_size(800, 600);
        window.set_title("Pipewire Graph Editor");
        window.add(&graphview.borrow().widget);
        window.show_all();
    }));

    app.run(&std::env::args().collect::<Vec<_>>());

    Ok(())
}
