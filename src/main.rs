mod view;

use glib::clone;
use gio::prelude::*;
use gtk::prelude::*;

use std::rc::Rc;

pub struct PipewireLink {
    pub node_from: u32,
    pub port_from: u32,
    pub node_to: u32,
    pub port_to: u32
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;
    let mut graphview = view::GraphView::new();

    // For UI Testing purposes
    let mut node = view::PipewireNode::new("Test Node");
    node.add_ingoing_port(10, gtk::Button::with_label("Ingoing Port"));
    node.add_outgoing_port(11, gtk::Button::with_label("Outgoing Port"));
    node.add_outgoing_port(12, gtk::Button::with_label("Outgoing Port 2"));

    let mut node2 = view::PipewireNode::new("Test Node 2");
    node2.add_ingoing_port(13, gtk::Button::with_label("Ingoing Port"));
    node2.add_outgoing_port(14, gtk::Button::with_label("Outgoing Port"));
    node2.add_outgoing_port(15, gtk::Button::with_label("Outgoing Port 2"));

    graphview.add_node(0, node);
    graphview.add_node(1, node2);
    graphview.add_link(2, PipewireLink {
        node_from: 0,
        port_from: 12,
        node_to: 1,
        port_to: 13
    });
    // End UI Testing

    let graphview = Rc::new(graphview);

    let app = gtk::Application::new(
        Some("org.freedesktop.pipewire.graphui"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .expect("Application creation failed");

    app.connect_activate(clone!(@strong graphview => move |app| {
        let window = gtk::ApplicationWindow::new(app);
        window.set_default_size(800, 600);
        window.set_title("Pipewire Graph Editor");
        window.add(&graphview.widget);
        window.show_all();
    }));

    app.run(&std::env::args().collect::<Vec<_>>());

    Ok(())
}
