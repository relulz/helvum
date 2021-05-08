use std::{cell::RefCell, collections::HashMap};

use gtk::{
    gio,
    glib::{self, clone, Continue, Receiver},
    prelude::*,
    subclass::prelude::*,
};
use log::{info, warn};
use pipewire::{channel::Sender, spa::Direction};

use crate::{
    view::{self},
    GtkMessage, PipewireLink, PipewireMessage,
};

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Midi,
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

    use once_cell::unsync::OnceCell;

    #[derive(Default)]
    pub struct Application {
        pub(super) graphview: view::GraphView,
        pub(super) state: RefCell<State>,
        pub(super) pw_sender: OnceCell<RefCell<Sender<GtkMessage>>>,
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
    pub(super) fn new(
        gtk_receiver: Receiver<PipewireMessage>,
        pw_sender: Sender<GtkMessage>,
    ) -> Self {
        let app: Application =
            glib::Object::new(&[("application-id", &"org.freedesktop.ryuukyu.helvum")])
                .expect("Failed to create new Application");

        let imp = imp::Application::from_instance(&app);
        imp.pw_sender
            .set(RefCell::new(pw_sender))
            // Discard the returned sender, as it does not implement `Debug`.
            .map_err(|_| ())
            .expect("pw_sender field was already set");

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
            if let Some(Item::Node { media_type, .. }) = imp.state.borrow().get(node_id) {
                media_type.to_owned()
            } else {
                warn!("Node not found for Port {}", id);
                None
            };

        // Save node_id so we can delete this port easily.
        imp.state.borrow_mut().insert(id, Item::Port { node_id });

        let port = view::Port::new(id, name.as_str(), direction, media_type);

        // Create or delete a link if the widget emits the "port-toggled" signal.
        if let Err(e) = port.connect_local(
            "port_toggled",
            false,
            clone!(@weak self as app => @default-return None, move |args| {
                // Args always look like this: &[widget, id_port_from, id_port_to]
                let port_from = args[1].get_some::<u32>().unwrap();
                let port_to = args[2].get_some::<u32>().unwrap();

                app.toggle_link(port_from, port_to);

                None
            }),
        ) {
            warn!("Failed to connect to \"port-toggled\" signal: {}", e);
        }

        imp.graphview.add_port(node_id, id, port);
    }

    /// Add a new link to the view.
    pub fn add_link(&self, id: u32, link: PipewireLink) {
        info!("Adding link to graph: id {}", id);

        let imp = imp::Application::from_instance(self);

        // FIXME: Links should be colored depending on the data they carry (video, audio, midi) like ports are.

        imp.state.borrow_mut().insert(
            id,
            Item::Link {
                output_port: link.port_from,
                input_port: link.port_to,
            },
        );

        // Update graph to contain the new link.
        imp.graphview.add_link(id, link);
    }

    // Toggle a link between the two specified ports on the remote pipewire server.
    fn toggle_link(&self, port_from: u32, port_to: u32) {
        let imp = imp::Application::from_instance(self);
        let sender = imp.pw_sender.get().expect("pw_sender not set").borrow_mut();
        let state = imp.state.borrow_mut();

        if let Some(id) = state.get_link_id(port_from, port_to) {
            info!("Requesting removal of link with id {}", id);

            sender
                .send(GtkMessage::DestroyGlobal(id))
                .expect("Failed to send message");
        } else {
            info!(
                "Requesting creation of link from port id:{} to port id:{}",
                port_from, port_to
            );

            let node_from = state
                .get_node_of_port(port_from)
                .expect("Requested port not in state");
            let node_to = state
                .get_node_of_port(port_to)
                .expect("Requested port not in state");

            sender
                .send(GtkMessage::CreateLink(PipewireLink {
                    node_from,
                    port_from,
                    node_to,
                    port_to,
                }))
                .expect("Failed to send message");
        }
    }

    /// Handle a global object being removed.
    pub fn remove_global(&self, id: u32) {
        let imp = imp::Application::from_instance(self);

        if let Some(item) = imp.state.borrow_mut().remove(id) {
            match item {
                Item::Node { .. } => self.remove_node(id),
                Item::Port { node_id } => self.remove_port(id, node_id),
                Item::Link { .. } => self.remove_link(id),
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

/// Any pipewire item we need to keep track of.
/// These will be saved in the [`Application`]s `state` struct associated with their id.
enum Item {
    Node {
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
    Link {
        output_port: u32,
        input_port: u32,
    },
}

/// This struct keeps track of any relevant items and stores them under their IDs.
///
/// Given two port ids, it can also efficiently find the id of the link that connects them.
#[derive(Default)]
struct State {
    /// Map pipewire ids to items.
    items: HashMap<u32, Item>,
    /// Map `(output port id, input port id)` tuples to the id of the link that connects them.
    links: HashMap<(u32, u32), u32>,
}

impl State {
    /// Add a new item under the specified id.
    fn insert(&mut self, id: u32, item: Item) {
        if let Item::Link {
            output_port,
            input_port,
        } = item
        {
            self.links.insert((output_port, input_port), id);
        }

        self.items.insert(id, item);
    }

    /// Get the item that has the specified id.
    fn get(&self, id: u32) -> Option<&Item> {
        self.items.get(&id)
    }

    /// Get the id of the link that links the two specified ports.
    fn get_link_id(&self, output_port: u32, input_port: u32) -> Option<u32> {
        self.links.get(&(output_port, input_port)).copied()
    }

    /// Remove the item with the specified id, returning it if it exists.
    fn remove(&mut self, id: u32) -> Option<Item> {
        let removed = self.items.remove(&id);

        if let Some(Item::Link {
            output_port,
            input_port,
        }) = removed
        {
            self.links.remove(&(output_port, input_port));
        }

        removed
    }

    /// Convenience function: Get the id of the node a port is on
    fn get_node_of_port(&self, port: u32) -> Option<u32> {
        if let Some(Item::Port { node_id }) = self.get(port) {
            Some(*node_id)
        } else {
            None
        }
    }
}
