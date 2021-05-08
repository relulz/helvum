use std::{cell::RefCell, collections::HashMap};

use gtk::{
    gio,
    glib::{self, clone, Continue, Receiver},
    prelude::*,
    subclass::prelude::*,
};
use log::{info, warn};
use pipewire::spa::Direction;

use crate::{
    view::{self},
    PipewireMessage,
};

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Midi,
}

/// Any pipewire item we need to keep track of.
/// These will be saved in the controllers `state` map associated with their id.
enum Item {
    Node {
        // Keep track of the widget to easily remove ports on it later.
        // widget: view::Node,
        // Keep track of the nodes media type to color ports on it.
        media_type: Option<MediaType>,
    },
    Port {
        // Save the id of the node this is on so we can remove the port from it
        // when it is deleted.
        node_id: u32,
    },
    // We don't need to memorize anything about links right now, but we need to
    // be able to find out an id is a link.
    Link,
}

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

    #[derive(Default)]
    pub struct Application {
        pub(super) graphview: view::GraphView,
        pub(super) state: RefCell<HashMap<u32, Item>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "HelvumApplication";
        type Type = super::Application;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for Application {}
    impl ApplicationImpl for Application {
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
    impl GtkApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    /// Create the view.
    /// This will set up the entire user interface and prepare it for being run.
    pub(super) fn new(gtk_receiver: Receiver<PipewireMessage>) -> Self {
        let app: Application =
            glib::Object::new(&[("application-id", &"org.freedesktop.ryuukyu.helvum")])
                .expect("Failed to create new Application");

        // Add <Control-Q> shortcut for quitting the application.
        let quit = gtk::gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(@weak app => move |_, _| {
            app.quit();
        }));
        app.set_accels_for_action("app.quit", &["<Control>Q"]);
        app.add_action(&quit);

        // React to messages received from the pipewire thread.
        gtk_receiver.attach(
            None,
            clone!(
                @weak app => @default-return Continue(true),
                move |msg| {
                    match msg {
                        PipewireMessage::NodeAdded {
                            id,
                            name,
                            media_type,
                        } => app.add_node(id, name, media_type),
                        PipewireMessage::PortAdded {
                            id,
                            node_id,
                            name,
                            direction,
                        } => app.add_port(id, name, node_id, direction),
                        PipewireMessage::LinkAdded { id, link } => app.add_link(id, link),
                        PipewireMessage::ObjectRemoved { id } => app.remove_global(id),
                    };
                    Continue(true)
                }
            ),
        );

        app
    }

    /// Add a new node to the view.
    pub fn add_node(&self, id: u32, name: String, media_type: Option<MediaType>) {
        info!("Adding node to graph: id {}", id);

        let imp = imp::Application::from_instance(self);

        imp.state.borrow_mut().insert(
            id,
            Item::Node {
                // widget: node_widget,
                media_type,
            },
        );

        imp.graphview.add_node(id, view::Node::new(name.as_str()));
    }

    /// Add a new port to the view.
    pub fn add_port(&self, id: u32, name: String, node_id: u32, direction: Direction) {
        info!("Adding port to graph: id {}", id);

        let imp = imp::Application::from_instance(self);

        // Find out the nodes media type so that the port can be colored.
        let media_type =
            if let Some(Item::Node { media_type, .. }) = imp.state.borrow().get(&node_id) {
                media_type.to_owned()
            } else {
                warn!("Node not found for Port {}", id);
                None
            };

        // Save node_id so we can delete this port easily.
        imp.state.borrow_mut().insert(id, Item::Port { node_id });

        let port = view::Port::new(id, name.as_str(), direction, media_type);
        imp.graphview.add_port(node_id, id, port);
    }

    /// Add a new link to the view.
    pub fn add_link(&self, id: u32, link: crate::PipewireLink) {
        info!("Adding link to graph: id {}", id);

        let imp = imp::Application::from_instance(self);

        // FIXME: Links should be colored depending on the data they carry (video, audio, midi) like ports are.

        imp.state.borrow_mut().insert(id, Item::Link);

        // Update graph to contain the new link.
        imp.graphview.add_link(id, link);
    }

    /// Handle a global object being removed.
    pub fn remove_global(&self, id: u32) {
        let imp = imp::Application::from_instance(self);

        if let Some(item) = imp.state.borrow_mut().remove(&id) {
            match item {
                Item::Node { .. } => self.remove_node(id),
                Item::Port { node_id } => self.remove_port(id, node_id),
                Item::Link => self.remove_link(id),
            }
        } else {
            warn!(
                "Attempted to remove item with id {} that is not saved in state",
                id
            );
        }
    }

    /// Remove the node with the specified id from the view.
    fn remove_node(&self, id: u32) {
        info!("Removing node from graph: id {}", id);

        let imp = imp::Application::from_instance(self);
        imp.graphview.remove_node(id);
    }

    /// Remove the port with the id `id` from the node with the id `node_id`
    /// from the view.
    fn remove_port(&self, id: u32, node_id: u32) {
        info!("Removing port from graph: id {}, node_id: {}", id, node_id);

        let imp = imp::Application::from_instance(self);
        imp.graphview.remove_port(id, node_id);
    }

    /// Remove the link with the specified id from the view.
    fn remove_link(&self, id: u32) {
        info!("Removing link from graph: id {}", id);

        let imp = imp::Application::from_instance(self);
        imp.graphview.remove_link(id);
    }
}
