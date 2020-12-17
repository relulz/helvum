use gtk::prelude::*;
use gdk::prelude::*;
use std::collections::HashMap;

pub struct PipewireNode {
    pub(super) widget: gtk::Grid,
    label: gtk::Label,
    label_box: gtk::EventBox,
    ingoing_ports: HashMap<u32, gtk::Button>,
    outgoing_ports: HashMap<u32, gtk::Button>,
}

impl PipewireNode {
    pub fn new(name: &str) -> Self {
        let result = Self {
            widget: gtk::Grid::new(),
            label: gtk::Label::new(Some(name)),
            label_box: gtk::EventBox::new(),
            ingoing_ports: HashMap::new(),
            outgoing_ports: HashMap::new(),
        };

        result.label_box.add(&result.label);
        result.widget.attach(&result.label_box, 0, 0, 2, 1);

        result
            .label_box
            .add_events(gdk::EventMask::BUTTON1_MOTION_MASK);
        result
            .label_box
            .connect_motion_notify_event(|label, event| {
                let grid = label
                    .get_ancestor(gtk::Grid::static_type())
                    .unwrap()
                    .dynamic_cast::<gtk::Grid>()
                    .unwrap();
                let graphview = grid
                    .get_ancestor(gtk::Layout::static_type())
                    .unwrap()
                    .dynamic_cast::<gtk::Layout>()
                    .unwrap();

                // Use root coordinates to prevent jumping around
                // as moving the widget also influences the relative coordinates.
                let (x, y) = event.get_root();
                let (offset_x, offset_y) = graphview.get_window().unwrap().get_root_origin();

                // TODO: Calculate proper values to center the mouse on the label
                // instead of using hardcoded offsets.
                graphview.set_child_x(&grid, x as i32 - offset_x - 100);
                graphview.set_child_y(&grid, y as i32 - offset_y - 50);

                // FIXME: If links become proper widgets,
                // we don't need to redraw the full graph everytime.
                graphview.queue_draw();

                Inhibit(true)
            });

        result
    }

    pub fn add_ingoing_port(&mut self, id: u32, port: gtk::Button) {
        self.widget
            .attach(&port, 0, (self.ingoing_ports.len() + 1) as i32, 1, 1);
        self.ingoing_ports.insert(id, port);
    }

    pub fn add_outgoing_port(&mut self, id: u32, port: gtk::Button) {
        self.widget
            .attach(&port, 1, (self.outgoing_ports.len() + 1) as i32, 1, 1);
        self.outgoing_ports.insert(id, port);
    }

    pub fn get_ingoing_port(&self, id: u32) -> Option<&gtk::Button> {
        self.ingoing_ports.get(&id)
    }

    pub fn get_outgoing_port(&self, id: u32) -> Option<&gtk::Button> {
        self.outgoing_ports.get(&id)
    }
}
