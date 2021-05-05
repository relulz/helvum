use gtk::glib;
use pipewire::{
    prelude::*,
    registry::GlobalObject,
    spa::{Direction, ForeignDict},
    types::ObjectType,
    Context, MainLoop,
};

use crate::{controller::MediaType, GtkMessage, PipewireMessage};

/// The "main" function of the pipewire thread.
pub(super) fn thread_main(
    gtk_sender: glib::Sender<PipewireMessage>,
    pw_receiver: pipewire::channel::Receiver<GtkMessage>,
) {
    let mainloop = MainLoop::new().expect("Failed to create mainloop");
    let context = Context::new(&mainloop).expect("Failed to create context");
    let core = context.connect(None).expect("Failed to connect to remote");
    let registry = core.get_registry().expect("Failed to get registry");

    let _receiver = pw_receiver.attach(&mainloop, {
        let mainloop = mainloop.clone();
        move |msg| match msg {
            GtkMessage::Terminate => mainloop.quit(),
        }
    });

    let _listener = registry
        .add_listener_local()
        .global({
            let sender = gtk_sender.clone();
            move |global| match global.type_ {
                ObjectType::Node => handle_node(global, &sender),
                ObjectType::Port => handle_port(global, &sender),
                ObjectType::Link => handle_link(global, &sender),
                _ => {
                    // Other objects are not interesting to us
                }
            }
        })
        .global_remove(move |id| {
            gtk_sender
                .send(PipewireMessage::ObjectRemoved { id })
                .expect("Failed to send message")
        })
        .register();

    mainloop.run();
}

/// Handle a new node being added
fn handle_node(node: &GlobalObject<ForeignDict>, sender: &glib::Sender<PipewireMessage>) {
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

    // FIXME: This relies on the node being passed to us by the pipwire server before its port.
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

    sender
        .send(PipewireMessage::NodeAdded {
            id: node.id,
            name,
            media_type,
        })
        .expect("Failed to send message");
}

/// Handle a new port being added
fn handle_port(port: &GlobalObject<ForeignDict>, sender: &glib::Sender<PipewireMessage>) {
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

    sender
        .send(PipewireMessage::PortAdded {
            id: port.id,
            node_id,
            name,
            direction,
        })
        .expect("Failed to send message");
}

/// Handle a new link being added
fn handle_link(link: &GlobalObject<ForeignDict>, sender: &glib::Sender<PipewireMessage>) {
    let props = link
        .props
        .as_ref()
        .expect("Link object is missing properties");
    let node_from: u32 = props
        .get("link.output.node")
        .expect("Link has no link.input.node property")
        .parse()
        .expect("Could not parse link.input.node property");
    let port_from: u32 = props
        .get("link.output.port")
        .expect("Link has no link.output.port property")
        .parse()
        .expect("Could not parse link.output.port property");
    let node_to: u32 = props
        .get("link.input.node")
        .expect("Link has no link.input.node property")
        .parse()
        .expect("Could not parse link.input.node property");
    let port_to: u32 = props
        .get("link.input.port")
        .expect("Link has no link.input.port property")
        .parse()
        .expect("Could not parse link.input.port property");

    sender
        .send(PipewireMessage::LinkAdded {
            id: link.id,
            link: crate::PipewireLink {
                node_from,
                port_from,
                node_to,
                port_to,
            },
        })
        .expect("Failed to send message");
}
