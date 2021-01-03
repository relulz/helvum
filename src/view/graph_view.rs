use super::Node;
use cairo::Context;
use glib::clone;
use gtk::{prelude::*, LayoutExt};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub struct GraphView {
    pub(crate) widget: gtk::Layout,
    nodes: Rc<RefCell<HashMap<u32, Node>>>,
    links: Rc<RefCell<HashMap<u32, crate::PipewireLink>>>,
}

impl GraphView {
    pub fn new() -> Self {
        let result = Self {
            widget: gtk::Layout::new::<gtk::Adjustment, gtk::Adjustment>(None, None),
            nodes: Rc::new(RefCell::new(HashMap::new())),
            links: Rc::new(RefCell::new(HashMap::new())),
        };

        result.widget.connect_draw(clone!(
        @weak result.nodes as nodes, @weak result.links as links => @default-panic,
        move |_, cr| {
            draw(nodes, links, cr);
            Inhibit(false)
        }));

        result
    }

    pub fn add_node(&mut self, id: u32, node: Node) {
        // TODO: Find a free position to put the widget at.
        self.widget.put(
            &node.widget,
            (self.nodes.borrow().len() / 4 * 400) as i32,
            (self.nodes.borrow().len() % 4 * 100) as i32,
        );
        node.widget.show_all();
        self.nodes.borrow_mut().insert(id, node);
    }

    pub fn add_port_to_node(&mut self, node_id: u32, port_id: u32, port: super::port::Port) {
        if let Some(node) = self.nodes.borrow_mut().get_mut(&node_id) {
            node.add_port(port_id, port);
        } else {
            // FIXME: Log this instead
            eprintln!(
                "Node with id {} not found when trying to add port with id {} to graph",
                node_id, port_id
            );
        }
    }

    /// Add a link to the graph.
    ///
    /// `add_link` takes three arguments: `link_id` is the id of the link as assigned by the pipewire server,
    /// `from` and `to` are the id's of the ingoing and outgoing port, respectively.
    pub fn add_link(&mut self, link_id: u32, link: crate::PipewireLink) {
        self.links.borrow_mut().insert(link_id, link);
        self.widget.queue_draw();
    }
}

fn draw(
    nodes: Rc<RefCell<HashMap<u32, Node>>>,
    links: Rc<RefCell<HashMap<u32, crate::PipewireLink>>>,
    cr: &Context,
) {
    cr.set_line_width(2.0);
    cr.set_source_rgb(255.0, 255.0, 255.0);
    cr.paint();
    cr.set_source_rgb(0.0, 0.0, 0.0);
    for link in links.borrow().values() {
        if let Some((from_alloc, to_alloc)) = get_allocs(nodes.clone(), link) {
            let from_x: f64 = (from_alloc.x + from_alloc.width).into();
            let from_y: f64 = (from_alloc.y + (from_alloc.height / 2)).into();
            cr.move_to(from_x, from_y);

            let to_x: f64 = to_alloc.x.into();
            let to_y: f64 = (to_alloc.y + (to_alloc.height / 2)).into();
            cr.curve_to(from_x + 75.0, from_y, to_x - 75.0, to_y, to_x, to_y);

            cr.stroke();
        } else {
            eprintln!("Could not get allocation of ports of link: {:?}", link);
        }
    }
}

fn get_allocs(
    nodes: Rc<RefCell<HashMap<u32, Node>>>,
    link: &crate::PipewireLink,
) -> Option<(gtk::Allocation, gtk::Allocation)> {
    println!();

    let from_alloc = &nodes
        .borrow()
        .get(&link.node_from)?
        .get_port(link.port_from)?
        .widget
        .get_allocation();
    let to_alloc = &nodes
        .borrow()
        .get(&link.node_to)?
        .get_port(link.port_to)?
        .widget
        .get_allocation();

    Some((from_alloc.to_owned(), to_alloc.to_owned()))
}
