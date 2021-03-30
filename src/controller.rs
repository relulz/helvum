use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gtk::glib::{self, clone};
use libspa::{ForeignDict, ReadableDict};
use log::{info, warn};
use pipewire::{port::Direction, registry::GlobalObject, types::ObjectType};

use crate::{pipewire_connection::PipewireConnection, view};

#[derive(Copy, Clone)]
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
    con: Rc<RefCell<PipewireConnection>>,
    state: HashMap<u32, Item>,
    view: Rc<view::View>,
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
        view: Rc<view::View>,
        con: Rc<RefCell<PipewireConnection>>,
    ) -> Rc<RefCell<Controller>> {
        let result = Rc::new(RefCell::new(Controller {
            con,
            view,
            state: HashMap::new(),
        }));

        result
            .borrow()
            .con
            .borrow_mut()
            .on_global_add(Some(Box::new(
                clone!(@weak result as this => move |global| {
                    this.borrow_mut().global_add(global);
                }),
            )));
        result
            .borrow()
            .con
            .borrow_mut()
            .on_global_remove(Some(Box::new(clone!(@weak result as this => move |id| {
                    this.borrow_mut().global_remove(id);
            }))));

        result
    }

    /// Handle a new global object being added.
    /// Relevant objects are displayed to the user and/or stored to the state.
    ///
    /// It is called from the `PipewireConnection` via callback.
    fn global_add(&mut self, global: &GlobalObject<ForeignDict>) {
        match global.type_ {
            ObjectType::Node => {
                self.add_node(global);
            }
            ObjectType::Port => {
                self.add_port(global);
            }
            ObjectType::Link => {
                self.add_link(global);
            }
            _ => {}
        }
    }

    /// Handle a node object being added.
    fn add_node(&mut self, node: &GlobalObject<ForeignDict>) {
        info!("Adding node to graph: id {}", node.id);

        // Get the nicest possible name for the node, using a fallback chain of possible name attributes.
        let node_name = &node
            .props
            .as_ref()
            .map(|dict| {
                String::from(
                    dict.get("node.nick")
                        .or_else(|| dict.get("node.description"))
                        .or_else(|| dict.get("node.name"))
                        .unwrap_or_default(),
                )
            })
            .unwrap_or_default();

        // FIXME: This relies on the node being passed to us by the pipwire server before its port.
        let media_type = node
            .props
            .as_ref()
            .map(|props| {
                props.get("media.class").map(|class| {
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
            })
            .flatten()
            .flatten();

        self.view.add_node(node.id, node_name);

        self.state.insert(
            node.id,
            Item::Node {
                // widget: node_widget,
                media_type,
            },
        );
    }

    /// Handle a port object being added.
    fn add_port(&mut self, port: &GlobalObject<ForeignDict>) {
        info!("Adding port to graph: id {}", port.id);

        // Update graph to contain the new port.
        let props = port
            .props
            .as_ref()
            .expect("Port object is missing properties");
        let port_label = props.get("port.name").unwrap_or_default().to_string();
        let node_id: u32 = props
            .get("node.id")
            .expect("Port has no node.id property!")
            .parse()
            .expect("Could not parse node.id property");

        // Find out the nodes media type so that the port can be colored.
        let media_type = if let Some(Item::Node { media_type, .. }) = self.state.get(&node_id) {
            media_type.to_owned()
        } else {
            warn!("Node not found for Port {}", port.id);
            None
        };

        self.view.add_port(
            node_id,
            port.id,
            &port_label,
            if matches!(props.get("port.direction"), Some("in")) {
                Direction::Input
            } else {
                Direction::Output
            },
            media_type,
        );

        // Save node_id so we can delete this port easily.
        self.state.insert(port.id, Item::Port { node_id });
    }

    /// Handle a link object being added.
    fn add_link(&mut self, link: &GlobalObject<ForeignDict>) {
        info!("Adding link to graph: id {}", link.id);

        // FIXME: Links should be colored depending on the data they carry (video, audio, midi) like ports are.

        self.state.insert(link.id, Item::Link);

        // Update graph to contain the new link.
        let props = link
            .props
            .as_ref()
            .expect("Link object is missing properties");
        let input_node: u32 = props
            .get("link.input.node")
            .expect("Link has no link.input.node property")
            .parse()
            .expect("Could not parse link.input.node property");
        let input_port: u32 = props
            .get("link.input.port")
            .expect("Link has no link.input.port property")
            .parse()
            .expect("Could not parse link.input.port property");
        let output_node: u32 = props
            .get("link.output.node")
            .expect("Link has no link.input.node property")
            .parse()
            .expect("Could not parse link.input.node property");
        let output_port: u32 = props
            .get("link.output.port")
            .expect("Link has no link.output.port property")
            .parse()
            .expect("Could not parse link.output.port property");
        self.view.add_link(
            link.id,
            crate::PipewireLink {
                node_from: output_node,
                port_from: output_port,
                node_to: input_node,
                port_to: input_port,
            },
        );
    }

    /// Handle a globalobject being removed.
    /// Relevant objects are removed from the view and/or the state.
    ///
    /// This is called from the `PipewireConnection` via callback.
    fn global_remove(&mut self, id: u32) {
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
