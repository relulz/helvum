mod controller;
mod pipewire_connection;
mod view;

use std::rc::Rc;

use controller::MediaType;
use gtk::glib::{self, PRIORITY_DEFAULT};
use pipewire::spa::Direction;

/// Messages used GTK thread to command the pipewire thread.
#[derive(Debug)]
enum GtkMessage {
    /// Quit the event loop and let the thread finish.
    Terminate,
}

/// Messages used pipewire thread to notify the GTK thread.
#[derive(Debug)]
enum PipewireMessage {
    /// A new node has appeared.
    NodeAdded {
        id: u32,
        name: String,
        media_type: Option<MediaType>,
    },
    /// A new port has appeared.
    PortAdded {
        id: u32,
        node_id: u32,
        name: String,
        direction: Direction,
    },
    /// A new link has appeared.
    LinkAdded { id: u32, link: PipewireLink },
    /// An object was removed
    ObjectRemoved { id: u32 },
}

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

    // Start the pipewire thread with channels in both directions.
    let (gtk_sender, gtk_receiver) = glib::MainContext::channel(PRIORITY_DEFAULT);
    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let pw_thread =
        std::thread::spawn(move || pipewire_connection::thread_main(gtk_sender, pw_receiver));

    let view = Rc::new(view::View::new());
    let _controller = controller::Controller::new(view.clone(), gtk_receiver);

    view.run();

    pw_sender
        .send(GtkMessage::Terminate)
        .expect("Failed to send message");

    pw_thread.join().expect("Pipewire thread panicked");

    Ok(())
}
