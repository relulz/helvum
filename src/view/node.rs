use gdk::prelude::*;
use gtk::prelude::*;
use std::collections::HashMap;

use pipewire::port::Direction;

pub struct Node {
    pub(super) widget: gtk::Grid,
    label: gtk::Label,
    label_box: gtk::EventBox,
    ports: HashMap<u32, super::port::Port>,
    num_ports_in: u32,
    num_ports_out: u32,
}

impl Node {
    pub fn new(name: &str) -> Self {
        let result = Self {
            widget: gtk::Grid::new(),
            label: gtk::Label::new(Some(name)),
            label_box: gtk::EventBox::new(),
            ports: HashMap::new(),
            num_ports_in: 0,
            num_ports_out: 0,
        };

        result.label_box.add(&result.label);
        result.widget.attach(&result.label_box, 0, 0, 2, 1);

        // Setup needed events for dragging a node.
        result
            .label_box
            .add_events(gdk::EventMask::BUTTON1_MOTION_MASK);

        // Setup callback for dragging the node.
        result
            .label_box
            .connect_motion_notify_event(|label, event| {
                let node_frame = label
                    .get_ancestor(gtk::Grid::static_type())
                    .unwrap()
                    .dynamic_cast::<gtk::Grid>()
                    .unwrap();
                let graphview = node_frame
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
                graphview.set_child_x(&node_frame, x as i32 - offset_x - 100);
                graphview.set_child_y(&node_frame, y as i32 - offset_y - 50);

                // FIXME: If links become proper widgets,
                // we don't need to redraw the full graph everytime.
                graphview.queue_draw();

                Inhibit(true)
            });

        result
    }

    pub fn add_port(&mut self, id: u32, port: super::port::Port) {
        match port.direction {
            Direction::Input => {
                self.widget
                    .attach(&port.widget, 0, (self.num_ports_in + 1) as i32, 1, 1);
                self.num_ports_in += 1;
            }
            Direction::Output => {
                self.widget
                    .attach(&port.widget, 1, (self.num_ports_out + 1) as i32, 1, 1);
                self.num_ports_out += 1;
            }
        }

        port.widget.show_all();
        self.ports.insert(id, port);
    }

    pub fn get_port(&self, id: u32) -> Option<&super::port::Port> {
        self.ports.get(&id)
    }
}
