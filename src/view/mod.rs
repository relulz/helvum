//! The view presented to the user.
//!
//! This module contains gtk widgets and helper struct needed to present the graphical user interface.

mod graph_view;
mod node;
pub mod port;

pub use graph_view::GraphView;
pub use node::Node;

use gtk::{
    gio,
    glib::{self, clone},
    prelude::*,
    subclass::prelude::ObjectSubclassExt,
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

mod imp {
    use super::*;
    use gtk::{glib, subclass::prelude::*};

    #[derive(Default)]
    pub struct View {
        pub(super) graphview: GraphView,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for View {
        const NAME: &'static str = "HelvumApplication";
        type Type = super::View;
        type ParentType = gtk::Application;

        fn new() -> Self {
            View {
                graphview: GraphView::new(),
            }
        }
    }

    impl ObjectImpl for View {}
    impl ApplicationImpl for View {
        fn activate(&self, app: &Self::Type) {
            let scrollwindow = gtk::ScrolledWindowBuilder::new()
                .child(&self.graphview)
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
        }

        fn startup(&self, app: &Self::Type) {
            self.parent_startup(app);

            // Load CSS from the STYLE variable.
            let provider = gtk::CssProvider::new();
            provider.load_from_data(STYLE.as_bytes());
            gtk::StyleContext::add_provider_for_display(
                &gtk::gdk::Display::get_default().expect("Error initializing gtk css provider."),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
    impl GtkApplicationImpl for View {}
}

glib::wrapper! {
    pub struct View(ObjectSubclass<imp::View>)
        @extends gio::Application, gtk::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}



impl View {
    /// Create the view.
    /// This will set up the entire user interface and prepare it for being run.
    pub(super) fn new() -> Self {
        let app: View = glib::Object::new(&[("application-id", &"org.freedesktop.ryuukyu.helvum")])
            .expect("Failed to create new Application");

        // Add <Control-Q> shortcut for quitting the application.
        let quit = gtk::gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(@weak app => move |_, _| {
            app.quit();
        }));
        app.set_accels_for_action("app.quit", &["<Control>Q"]);
        app.add_action(&quit);

        app
    }

    /// Add a new node to the view.
    pub fn add_node(&self, id: u32, name: &str) {
        let imp = imp::View::from_instance(self);
        imp.graphview.add_node(id, crate::view::Node::new(name));
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
        let imp = imp::View::from_instance(self);
        imp.graphview.add_port(
            node_id,
            port_id,
            port::Port::new(port_id, port_name, port_direction, port_media_type),
        );
    }

    /// Add a new link to the view.
    pub fn add_link(&self, id: u32, link: crate::PipewireLink) {
        let imp = imp::View::from_instance(self);
        imp.graphview.add_link(id, link);
    }

    /// Remove the node with the specified id from the view.
    pub fn remove_node(&self, id: u32) {
        let imp = imp::View::from_instance(self);
        imp.graphview.remove_node(id);
    }

    /// Remove the port with the id `id` from the node with the id `node_id`
    /// from the view.
    pub fn remove_port(&self, id: u32, node_id: u32) {
        let imp = imp::View::from_instance(self);
        imp.graphview.remove_port(id, node_id);
    }

    /// Remove the link with the specified id from the view.
    pub fn remove_link(&self, id: u32) {
        let imp = imp::View::from_instance(self);
        imp.graphview.remove_link(id);
    }
}
