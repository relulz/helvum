use std::cell::RefCell;

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
    GtkMessage, MediaType, PipewireLink, PipewireMessage,
};

static STYLE: &str = include_str!("style.css");

mod imp {
    use super::*;

    use once_cell::unsync::OnceCell;

    #[derive(Default)]
    pub struct Application {
        pub(super) graphview: view::GraphView,
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
                .settings()
                .set_gtk_application_prefer_dark_theme(true);
            window.show();
        }

        fn startup(&self, app: &Self::Type) {
            self.parent_startup(app);

            // Load CSS from the STYLE variable.
            let provider = gtk::CssProvider::new();
            provider.load_from_data(STYLE.as_bytes());
            gtk::StyleContext::add_provider_for_display(
                &gtk::gdk::Display::default().expect("Error initializing gtk css provider."),
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
                        PipewireMessage::NodeAdded{ id, name } => app.add_node(id, name.as_str()),
                        PipewireMessage::PortAdded{ id, node_id, name, direction, media_type } => app.add_port(id, name.as_str(), node_id, direction, media_type),
                        PipewireMessage::LinkAdded{ id, node_from, port_from, node_to, port_to, active} => app.add_link(id, node_from, port_from, node_to, port_to, active),
                        PipewireMessage::LinkStateChanged { id, active } => app.link_state_changed(id, active), // TODO
                        PipewireMessage::NodeRemoved { id } => app.remove_node(id),
                        PipewireMessage::PortRemoved { id, node_id } => app.remove_port(id, node_id),
                        PipewireMessage::LinkRemoved { id } => app.remove_link(id)
                    };
                    Continue(true)
                }
            ),
        );

        app
    }

    /// Add a new node to the view.
    fn add_node(&self, id: u32, name: &str) {
        info!("Adding node to graph: id {}", id);

        imp::Application::from_instance(self)
            .graphview
            .add_node(id, view::Node::new(name));
    }

    /// Add a new port to the view.
    fn add_port(
        &self,
        id: u32,
        name: &str,
        node_id: u32,
        direction: Direction,
        media_type: Option<MediaType>,
    ) {
        info!("Adding port to graph: id {}", id);

        let imp = imp::Application::from_instance(self);

        let port = view::Port::new(id, name, direction, media_type);

        // Create or delete a link if the widget emits the "port-toggled" signal.
        if let Err(e) = port.connect_local(
            "port_toggled",
            false,
            clone!(@weak self as app => @default-return None, move |args| {
                // Args always look like this: &[widget, id_port_from, id_port_to]
                let port_from = args[1].get::<u32>().unwrap();
                let port_to = args[2].get::<u32>().unwrap();

                app.toggle_link(port_from, port_to);

                None
            }),
        ) {
            warn!("Failed to connect to \"port-toggled\" signal: {}", e);
        }

        imp.graphview.add_port(node_id, id, port);
    }

    /// Add a new link to the view.
    fn add_link(
        &self,
        id: u32,
        node_from: u32,
        port_from: u32,
        node_to: u32,
        port_to: u32,
        active: bool,
    ) {
        info!("Adding link to graph: id {}", id);

        // FIXME: Links should be colored depending on the data they carry (video, audio, midi) like ports are.

        // Update graph to contain the new link.
        imp::Application::from_instance(self).graphview.add_link(
            id,
            PipewireLink {
                node_from,
                port_from,
                node_to,
                port_to,
            },
            active,
        );
    }

    fn link_state_changed(&self, id: u32, active: bool) {
        info!(
            "Link state changed: Link (id={}) is now {}",
            id,
            if active { "active" } else { "inactive" }
        );

        imp::Application::from_instance(self)
            .graphview
            .set_link_state(id, active);
    }

    // Toggle a link between the two specified ports on the remote pipewire server.
    fn toggle_link(&self, port_from: u32, port_to: u32) {
        let imp = imp::Application::from_instance(self);
        let sender = imp.pw_sender.get().expect("pw_sender not set").borrow_mut();
        sender
            .send(GtkMessage::ToggleLink { port_from, port_to })
            .expect("Failed to send message");
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
