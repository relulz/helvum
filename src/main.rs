mod application;
mod pipewire_connection;
mod view;

use gtk::{
    glib::{self, PRIORITY_DEFAULT},
    prelude::*,
};
use pipewire::spa::Direction;

/// Messages sent by the GTK thread to notify the pipewire thread.
#[derive(Debug, Clone)]
enum GtkMessage {
    /// Toggle a link between the two specified ports.
    ToggleLink { port_from: u32, port_to: u32 },
    /// Quit the event loop and let the thread finish.
    Terminate,
}

/// Messages sent by the pipewire thread to notify the GTK thread.
#[derive(Debug, Clone)]
enum PipewireMessage {
    NodeAdded {
        id: u32,
        name: String,
    },
    PortAdded {
        id: u32,
        node_id: u32,
        name: String,
        direction: Direction,
        media_type: Option<MediaType>,
    },
    LinkAdded {
        id: u32,
        node_from: u32,
        port_from: u32,
        node_to: u32,
        port_to: u32,
        active: bool,
    },
    LinkStateChanged {
        id: u32,
        active: bool,
    },
    NodeRemoved {
        id: u32,
    },
    PortRemoved {
        id: u32,
        node_id: u32,
    },
    LinkRemoved {
        id: u32,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Midi,
}

#[derive(Debug, Clone)]
pub struct PipewireLink {
    pub node_from: u32,
    pub port_from: u32,
    pub node_to: u32,
    pub port_to: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    gtk::init()?;

    // Aquire main context so that we can attach the gtk channel later.
    let ctx = glib::MainContext::default();
    let _guard = ctx.acquire().unwrap();

    // Start the pipewire thread with channels in both directions.
    let (gtk_sender, gtk_receiver) = glib::MainContext::channel(PRIORITY_DEFAULT);
    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let pw_thread =
        std::thread::spawn(move || pipewire_connection::thread_main(gtk_sender, pw_receiver));

    let app = application::Application::new(gtk_receiver, pw_sender.clone());

    app.run();

    pw_sender
        .send(GtkMessage::Terminate)
        .expect("Failed to send message");

    pw_thread.join().expect("Pipewire thread panicked");

    Ok(())
}
