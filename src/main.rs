mod controller;
mod pipewire_connection;
mod view;

use std::rc::Rc;

use gtk::{glib, prelude::*};

#[derive(Debug)]
pub struct PipewireLink {
    pub node_from: u32,
    pub port_from: u32,
    pub node_to: u32,
    pub port_to: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    gtk::init()?;

    let view = Rc::new(view::View::new());
    let pw_con = pipewire_connection::PipewireConnection::new()?;
    let _controller = controller::Controller::new(view.clone(), pw_con.clone());

    // Do an initial roundtrip before showing the view,
    // so that the graph is already populated when the window opens.
    pw_con.borrow().roundtrip();
    // From now on, call roundtrip() every second.
    glib::timeout_add_seconds_local(1, move || {
        pw_con.borrow().roundtrip();
        Continue(true)
    });

    view.run();

    Ok(())
}
