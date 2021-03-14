mod pipewire_connection;
mod pipewire_state;
mod view;

use gtk::glib::{self, clone};
use gtk::prelude::*;

use std::{cell::RefCell, rc::Rc};

// FIXME: This should be in its own .css file.
static STYLE: &str = "
.audio {
    background: rgb(50,100,240);
	color: black;
}

.video {
    background: rgb(200,200,0);
	color: black;
}

.midi {
    background: rgb(200,0,50);
    color: black;
}
";

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

    let app = gtk::Application::new(Some("org.freedesktop.ryuukyu.helvum"), Default::default())
        .expect("Application creation failed");

    app.connect_startup(|_| {
        // Load CSS from the STYLE variable.
        let provider = gtk::CssProvider::new();
        provider.load_from_data(STYLE.as_bytes());
        gtk::StyleContext::add_provider_for_display(
            &gtk::gdk::Display::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(move |app| {
        let scrollwindow = gtk::ScrolledWindowBuilder::new()
            .child(&*graphview.borrow())
            .build();
        let window = gtk::ApplicationWindowBuilder::new()
            .application(app)
            .default_width(1280)
            .default_height(720)
            .title("Helvum - Pipewire Patchbay")
            .child(&scrollwindow)
            .build();
        window
            .get_settings()
            .set_property_gtk_application_prefer_dark_theme(true);
        window.show();
    });

    let quit = gtk::gio::SimpleAction::new("quit", None);
    quit.connect_activate(clone!(@weak app => move |_, _| {
        app.quit();
    }));
    app.set_accels_for_action("app.quit", &["<Control>Q"]);
    app.add_action(&quit);

    app.run(&std::env::args().collect::<Vec<_>>());

    Ok(())
}
