use gtk::GridExt;

use std::collections::HashMap;

pub struct PipewireNode {
    pub(super) widget: gtk::Grid,
    _label: gtk::Label,
    ingoing_ports: HashMap<u32, gtk::Button>,
    outgoing_ports: HashMap<u32, gtk::Button>,
}

impl PipewireNode {
    pub fn new(name: &str) -> Self {
        let widget = gtk::Grid::new();
        let label = gtk::Label::new(Some(name));
        widget.attach(&label, 0, 0, 2, 1);

        Self {
            widget,
            _label: label,
            ingoing_ports: HashMap::new(),
            outgoing_ports: HashMap::new(),
        }
    }

    pub fn add_ingoing_port(&mut self, id: u32, port: gtk::Button) {
        self.widget.attach(&port, 0, (self.ingoing_ports.len() + 1) as i32, 1, 1);
        self.ingoing_ports.insert(id, port);
    }

    pub fn add_outgoing_port(&mut self, id: u32, port: gtk::Button) {
        self.widget.attach(&port, 1, (self.outgoing_ports.len() + 1) as i32, 1, 1);
        self.outgoing_ports.insert(id, port);
    }

    pub fn get_ingoing_port(&self, id: u32) -> Option<&gtk::Button> {
        self.ingoing_ports.get(&id)
    }

    pub fn get_outgoing_port(&self, id: u32) -> Option<&gtk::Button> {
        self.outgoing_ports.get(&id)
    }
}
