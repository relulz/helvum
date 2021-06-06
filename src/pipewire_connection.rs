use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gtk::glib::{self, clone};
use log::{error, info, warn};
use pipewire::{
    link::Link,
    prelude::*,
    properties,
    registry::{GlobalObject, Registry},
    spa::{Direction, ForeignDict},
    types::ObjectType,
    Context, Core, MainLoop,
};

use crate::{GtkMessage, MediaType, PipewireMessage};

/// The "main" function of the pipewire thread.
pub(super) fn thread_main(
    gtk_sender: glib::Sender<PipewireMessage>,
    pw_receiver: pipewire::channel::Receiver<GtkMessage>,
) {
    let mainloop = MainLoop::new().expect("Failed to create mainloop");
    let context = Context::new(&mainloop).expect("Failed to create context");
    let core = Rc::new(context.connect(None).expect("Failed to connect to remote"));
    let registry = Rc::new(core.get_registry().expect("Failed to get registry"));

    let state = Rc::new(RefCell::new(State::new()));

    let _receiver = pw_receiver.attach(&mainloop, {
        clone!(@strong mainloop, @weak core, @weak registry, @strong state => move |msg| match msg {
            GtkMessage::ToggleLink { port_from, port_to } => toggle_link(port_from, port_to, &core, &registry, &state),
            GtkMessage::Terminate => mainloop.quit(),
        })
    });

    let _listener = registry
        .add_listener_local()
        .global(clone!(@strong gtk_sender, @strong state =>
            move |global| match global.type_ {
                ObjectType::Node => handle_node(global, &gtk_sender, &state),
                ObjectType::Port => handle_port(global, &gtk_sender, &state),
                ObjectType::Link => handle_link(global, &gtk_sender, &state),
                _ => {
                    // Other objects are not interesting to us
                }
            }
        ))
        .global_remove(clone!(@strong state => move |id| {
            if let Some(item) = state.borrow_mut().remove(id) {
                gtk_sender.send(match item {
                    Item::Node { .. } => PipewireMessage::NodeRemoved {id},
                    Item::Port { node_id } => PipewireMessage::PortRemoved {id, node_id},
                    Item::Link { .. } => PipewireMessage::LinkRemoved {id},
                }).expect("Failed to send message");
            } else {
                warn!(
                    "Attempted to remove item with id {} that is not saved in state",
                    id
                );
            }
        }))
        .register();

    mainloop.run();
}

/// Handle a new node being added
fn handle_node(
    node: &GlobalObject<ForeignDict>,
    sender: &glib::Sender<PipewireMessage>,
    state: &Rc<RefCell<State>>,
) {
    let props = node
        .props
        .as_ref()
        .expect("Node object is missing properties");

    // Get the nicest possible name for the node, using a fallback chain of possible name attributes.
    let name = String::from(
        props
            .get("node.nick")
            .or_else(|| props.get("node.description"))
            .or_else(|| props.get("node.name"))
            .unwrap_or_default(),
    );

    // FIXME: Instead of checking these props, the "EnumFormat" parameter should be checked instead.
    let media_type = props
        .get("media.class")
        .map(|class| {
            if class.contains("Audio") {
                Some(MediaType::Audio)
            } else if class.contains("Video") {
                Some(MediaType::Video)
            } else if class.contains("Midi") {
                Some(MediaType::Midi)
            } else {
                None
            }
        })
        .flatten();

    state.borrow_mut().insert(
        node.id,
        Item::Node {
            // widget: node_widget,
            media_type,
        },
    );

    sender
        .send(PipewireMessage::NodeAdded { id: node.id, name })
        .expect("Failed to send message");
}

/// Handle a new port being added
fn handle_port(
    port: &GlobalObject<ForeignDict>,
    sender: &glib::Sender<PipewireMessage>,
    state: &Rc<RefCell<State>>,
) {
    let props = port
        .props
        .as_ref()
        .expect("Port object is missing properties");
    let name = props.get("port.name").unwrap_or_default().to_string();
    let node_id: u32 = props
        .get("node.id")
        .expect("Port has no node.id property!")
        .parse()
        .expect("Could not parse node.id property");
    let direction = if matches!(props.get("port.direction"), Some("in")) {
        Direction::Input
    } else {
        Direction::Output
    };

    // Find out the nodes media type so that the port can be colored.
    let media_type = if let Some(Item::Node { media_type, .. }) = state.borrow().get(node_id) {
        media_type.to_owned()
    } else {
        warn!("Node not found for Port {}", port.id);
        None
    };

    // Save node_id so we can delete this port easily.
    state.borrow_mut().insert(port.id, Item::Port { node_id });

    sender
        .send(PipewireMessage::PortAdded {
            id: port.id,
            node_id,
            name,
            direction,
            media_type,
        })
        .expect("Failed to send message");
}

/// Handle a new link being added
fn handle_link(
    link: &GlobalObject<ForeignDict>,
    sender: &glib::Sender<PipewireMessage>,
    state: &Rc<RefCell<State>>,
) {
    let props = link
        .props
        .as_ref()
        .expect("Link object is missing properties");
    let port_from: u32 = props
        .get("link.output.port")
        .expect("Link has no link.output.port property")
        .parse()
        .expect("Could not parse link.output.port property");
    let port_to: u32 = props
        .get("link.input.port")
        .expect("Link has no link.input.port property")
        .parse()
        .expect("Could not parse link.input.port property");

    let mut state = state.borrow_mut();
    let node_from = *match state.get(port_from) {
        Some(Item::Port { node_id }) => node_id,
        _ => {
            error!(
                "Tried to add link (id:{}), but its output port (id:{}) is not known",
                link.id, port_from
            );
            return;
        }
    };
    let node_to = *match state.get(port_to) {
        Some(Item::Port { node_id }) => node_id,
        _ => {
            error!(
                "Tried to add link (id:{}), but its input port (id:{}) is not known",
                link.id, port_to
            );
            return;
        }
    };

    state.insert(
        link.id,
        Item::Link {
            output_port: port_from,
            input_port: port_to,
        },
    );

    sender
        .send(PipewireMessage::LinkAdded {
            id: link.id,
            node_from,
            port_from,
            node_to,
            port_to,
        })
        .expect("Failed to send message");
}

/// Toggle a link between the two specified ports.
fn toggle_link(
    port_from: u32,
    port_to: u32,
    core: &Rc<Core>,
    registry: &Rc<Registry>,
    state: &Rc<RefCell<State>>,
) {
    let state = state.borrow_mut();
    if let Some(id) = state.get_link_id(port_from, port_to) {
        info!("Requesting removal of link with id {}", id);

        // FIXME: Handle error
        registry.destroy_global(id);
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

        if let Err(e) = core.create_object::<Link, _>(
            "link-factory",
            &properties! {
                "link.output.node" => node_from.to_string(),
                "link.output.port" => port_from.to_string(),
                "link.input.node" => node_to.to_string(),
                "link.input.port" => port_to.to_string(),
                "object.linger" => "1"
            },
        ) {
            warn!("Failed to create link: {}", e);
        }
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
    /// Create a new, empty state.
    fn new() -> Self {
        Default::default()
    }

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
