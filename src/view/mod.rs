//! The view presented to the user.
//!
//! This module contains gtk widgets and helper struct needed to present the graphical user interface.

mod graph_view;
mod node;
pub mod port;

pub use graph_view::GraphView;
pub use node::Node;

use gtk::{
    glib::{self, clone},
    prelude::*,
};
use pipewire::spa::Direction;

use crate::controller::MediaType;

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

/// Manager struct of the view.
///
/// This struct is responsible for setting up the UI, as well as communication with other components outside the view.
pub struct View {
    app: gtk::Application,
    graphview: GraphView,
}

impl View {
    /// Create the view.
    /// This will set up the entire user interface and prepare it for being run.
    ///
    /// To show and run the interface, its [`run`](`Self::run`) method will need to be called.
    pub(super) fn new() -> Self {
        let graphview = GraphView::new();

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

        app.connect_activate(clone!(@strong graphview => move |app| {
            let scrollwindow = gtk::ScrolledWindowBuilder::new().child(&graphview).build();
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
        }));

        // Add <Control-Q> shortcut for quitting the application.
        let quit = gtk::gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(@weak app => move |_, _| {
            app.quit();
        }));
        app.set_accels_for_action("app.quit", &["<Control>Q"]);
        app.add_action(&quit);

        Self { app, graphview }
    }

    /// Run the view.
    ///
    /// This will enter a gtk event loop and remain in that
    /// until the application is quit by the user.
    pub(super) fn run(&self) -> i32 {
        self.app.run(&std::env::args().collect::<Vec<_>>())
    }

    /// Add a new node to the view.
    pub fn add_node(&self, id: u32, name: &str) {
        let node = crate::view::Node::new(name);
        self.graphview.add_node(id, node);
    }

    /// Add a new port to the view.
    pub fn add_port(
        &self,
        node_id: u32,
        port_id: u32,
        port_name: &str,
        port_direction: Direction,
        port_media_type: Option<MediaType>,
    ) {
        let port = port::Port::new(port_id, port_name, port_direction, port_media_type);
        self.graphview.add_port(node_id, port_id, port)
    }

    /// Add a new link to the view.
    pub fn add_link(&self, id: u32, link: crate::PipewireLink) {
        self.graphview.add_link(id, link);
    }

    /// Remove the node with the specified id from the view.
    pub fn remove_node(&self, id: u32) {
        self.graphview.remove_node(id);
    }

    /// Remove the port with the id `id` from the node with the id `node_id`
    /// from the view.
    pub fn remove_port(&self, id: u32, node_id: u32) {
        self.graphview.remove_port(id, node_id);
    }

    /// Remove the link with the specified id from the view.
    pub fn remove_link(&self, id: u32) {
        self.graphview.remove_link(id);
    }
}
