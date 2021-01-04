use crate::PipewireLink;

use pipewire as pw;
use pw::{port::Direction, registry::ObjectType, PW_ID_CORE};

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct PipewireConnection {
    mainloop: pw::MainLoop,
    _context: pw::Context<pw::MainLoop>,
    core: pw::Core,
    _registry: pw::registry::Registry,
    _reg_listeners: pw::registry::Listener,
}

impl PipewireConnection {
    pub fn new(graphview: Rc<RefCell<crate::view::GraphView>>) -> Result<Self, String> {
        pw::init();
        let mainloop = pw::MainLoop::new().map_err(|_| "Failed to create pipewire mainloop!")?;
        let context =
            pw::Context::new(&mainloop).map_err(|_| "Failed to create pipewire context")?;
        let core = context
            .connect()
            .map_err(|_| "Failed to connect to pipewire core")?;
        let registry = core.get_registry();

        let graphview = Rc::downgrade(&graphview.clone());
        let reg_listeners = registry
            .add_listener_local()
            .global(move |global| {
                PipewireConnection::handle_global(graphview.upgrade().unwrap(), global)
            })
            .global_remove(|_| { /* TODO */ })
            .register();

        Ok(Self {
            mainloop,
            _context: context,
            core,
            _registry: registry,
            _reg_listeners: reg_listeners,
        })
    }

    pub fn roundtrip(&self) {
        let done = Rc::new(Cell::new(false));
        let pending = self.core.sync(0);

        let done_clone = done.clone();
        let loop_clone = self.mainloop.clone();

        let _listener = self
            .core
            .add_listener_local()
            .done(move |id, seq| {
                if id == PW_ID_CORE && seq == pending {
                    done_clone.set(true);
                    loop_clone.quit();
                }
            })
            .register();

        while !done.get() {
            self.mainloop.run();
        }
    }

    fn handle_global(
        graphview: Rc<RefCell<crate::view::GraphView>>,
        global: pw::registry::GlobalObject,
    ) {
        match global.type_ {
            ObjectType::Node => {
                let node_widget = crate::view::Node::new(&format!(
                    "{}",
                    global
                        .props
                        .map(|dict| String::from(
                            dict.get("node.nick")
                                .or(dict.get("node.description"))
                                .or(dict.get("node.name"))
                                .unwrap_or_default()
                        ))
                        .unwrap_or_default()
                ));

                graphview.borrow_mut().add_node(global.id, node_widget);
            }
            ObjectType::Port => {
                let props = global.props.expect("Port object is missing properties");
                let port_label = format!("{}", props.get("port.name").unwrap_or_default());
                let node_id: u32 = props
                    .get("node.id")
                    .expect("Port has no node.id property!")
                    .parse()
                    .expect("Could not parse node.id property");
                let port = crate::view::port::Port::new(
                    global.id,
                    &port_label,
                    if matches!(props.get("port.direction"), Some("in")) {
                        Direction::Input
                    } else {
                        Direction::Output
                    },
                );

                graphview
                    .borrow_mut()
                    .add_port_to_node(node_id, global.id, port);
            }
            ObjectType::Link => {
                let props = global.props.expect("Link object is missing properties");
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
                graphview.borrow_mut().add_link(
                    global.id,
                    PipewireLink {
                        node_from: output_node,
                        port_from: output_port,
                        node_to: input_node,
                        port_to: input_port,
                    },
                );
            }
            _ => {}
        }
    }
}

/*impl Drop for PipewireConnection {
    fn drop(&mut self) {
        unsafe { pw::deinit() }
    }
}*/
