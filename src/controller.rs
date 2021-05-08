use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gtk::glib::{self, clone, Continue, Receiver};
use log::{info, warn};
use pipewire::spa::Direction;

use crate::{view, PipewireLink, PipewireMessage};

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

/// Mediater between the pipewire connection and the view.
///
/// The Controller is the central piece of the architecture.
/// It manages the view, receives updates from the pipewire connection
/// and relays changes the user made to the pipewire connection.
///
/// It also keeps and manages a state object that contains the current state of objects present on the remote.
pub struct Controller {
    state: HashMap<u32, Item>,
    view: view::View,
}

impl Controller {
    /// Create a new controller.
    ///
    /// This function returns an `Rc`, because `Weak` references are needed inside closures the controller
    /// passes to other components.
    ///
    /// The returned `Rc` will be the only strong reference kept to the controller, so dropping the `Rc`
    /// will also drop the controller, unless the `Rc` is cloned outside of this function.
    pub(super) fn new(
        view: view::View,
        gtk_receiver: Receiver<PipewireMessage>,
    ) -> Rc<RefCell<Controller>> {
        let result = Rc::new(RefCell::new(Controller {
            view,
            state: HashMap::new(),
        }));

        // React to messages received from the pipewire thread.
        gtk_receiver.attach(
            None,
            clone!(
                @weak result as controller => @default-return Continue(true),
                move |msg| {
                    match msg {
                        PipewireMessage::NodeAdded {
                            id,
                            name,
                            media_type,
                        } => controller.borrow_mut().add_node(id, name, media_type),
                        PipewireMessage::PortAdded {
                            id,
                            node_id,
                            name,
                            direction,
                        } => controller
                            .borrow_mut()
                            .add_port(id, node_id, name, direction),
                        PipewireMessage::LinkAdded { id, link } => controller.borrow_mut().add_link(id, link),
                        PipewireMessage::ObjectRemoved { id } => controller.borrow_mut().remove_global(id),
                    };
                    Continue(true)
                }
            )
        );

        result
    }

    /// Handle a node object being added.
    pub(super) fn add_node(&mut self, id: u32, name: String, media_type: Option<MediaType>) {
        info!("Adding node to graph: id {}", id);

        self.view.add_node(id, name.as_str());

        self.state.insert(
            id,
            Item::Node {
                // widget: node_widget,
                media_type,
            },
        );
    }

    /// Handle a port object being added.
    pub(super) fn add_port(&mut self, id: u32, node_id: u32, name: String, direction: Direction) {
        info!("Adding port to graph: id {}", id);

        // Update graph to contain the new port.

        // Find out the nodes media type so that the port can be colored.
        let media_type = if let Some(Item::Node { media_type, .. }) = self.state.get(&node_id) {
            media_type.to_owned()
        } else {
            warn!("Node not found for Port {}", id);
            None
        };

        self.view
            .add_port(node_id, id, &name, direction, media_type);

        // Save node_id so we can delete this port easily.
        self.state.insert(id, Item::Port { node_id });
    }

    /// Handle a link object being added.
    pub(super) fn add_link(&mut self, id: u32, link: PipewireLink) {
        info!("Adding link to graph: id {}", id);

        // FIXME: Links should be colored depending on the data they carry (video, audio, midi) like ports are.

        self.state.insert(id, Item::Link);

        // Update graph to contain the new link.
        self.view.add_link(id, link);
    }

    /// Handle a globalobject being removed.
    /// Relevant objects are removed from the view and/or the state.
    ///
    /// This is called from the `PipewireConnection` via callback.
    pub(super) fn remove_global(&mut self, id: u32) {
        if let Some(item) = self.state.remove(&id) {
            match item {
                Item::Node { .. } => {
                    info!("Removing node from graph: id {}", id);
                    self.view.remove_node(id);
                }
                Item::Port { node_id } => {
                    info!("Removing port from graph: id {}, node_id: {}", id, node_id);
                    self.view.remove_port(id, node_id);
                }
                Item::Link => {
                    info!("Removing link from graph: id {}", id);
                    self.view.remove_link(id);
                }
            }
        } else {
            warn!(
                "Attempted to remove item with id {} that is not saved in state",
                id
            );
        }
    }
}
