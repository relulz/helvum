use super::graph_view::GraphView;

use gtk::prelude::*;
use pipewire::port::Direction;

use std::collections::HashMap;

pub struct Node {
    pub(super) widget: gtk::Grid,
    label: gtk::Label,
    ports: HashMap<u32, super::port::Port>,
    num_ports_in: u32,
    num_ports_out: u32,
}

impl Node {
    pub fn new(name: &str) -> Self {
        let result = Self {
            widget: gtk::Grid::new(),
            label: gtk::Label::new(Some(name)),
            ports: HashMap::new(),
            num_ports_in: 0,
            num_ports_out: 0,
        };

        let motion_controller = gtk::EventControllerMotion::new();
        // Tell the graphview that the Node is the target of a drag when the mouse enters its label
        motion_controller.connect_enter(|controller, _, _| {
            let widget = controller
                .get_widget()
                .expect("Controller with enter event has no widget")
                .get_ancestor(gtk::Grid::static_type())
                .unwrap();
            widget
                .get_ancestor(GraphView::static_type())
                .unwrap()
                .dynamic_cast::<GraphView>()
                .unwrap()
                .set_dragged(Some(widget));
        });
        // Tell the graphview that the Node is no longer the target of a drag when the mouse leaves.
        motion_controller.connect_leave(|controller| {
            // FIXME: Check that we are the current target before setting none.
            controller
                .get_widget()
                .expect("Controller with leave event has no widget")
                .get_ancestor(GraphView::static_type())
                .unwrap()
                .dynamic_cast::<GraphView>()
                .unwrap()
                .set_dragged(None);
        });
        result.label.add_controller(&motion_controller);

        // Display a grab cursor when the mouse is over the label so the user knows the node can be dragged.
        result
            .label
            .set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

        result.widget.attach(&result.label, 0, 0, 2, 1);

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

        self.ports.insert(id, port);
    }

    pub fn get_port(&self, id: u32) -> Option<&super::port::Port> {
        self.ports.get(&id)
    }
}
