use super::PipewireNode;
use cairo::Context;
use glib::clone;
use gtk::{prelude::*, LayoutExt};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub struct GraphView {
    pub(crate) widget: gtk::Layout,
    nodes: Rc<RefCell<HashMap<u32, PipewireNode>>>,
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

    pub fn add_node(&mut self, id: u32, node: PipewireNode) {
        // TODO: Find a free position to put the widget at.
        self.widget.put(
            &node.widget,
            (self.nodes.borrow().len() % 4 * 400) as i32,
            (self.nodes.borrow().len() / 4 * 100) as i32,
        );
        self.nodes.borrow_mut().insert(id, node);
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
    nodes: Rc<RefCell<HashMap<u32, PipewireNode>>>,
    links: Rc<RefCell<HashMap<u32, crate::PipewireLink>>>,
    cr: &Context,
) {
    cr.set_line_width(2.0);
    cr.set_source_rgb(255.0, 255.0, 255.0);
    cr.paint();
    cr.set_source_rgb(0.0, 0.0, 0.0);
    for link in links.borrow().values() {
        let (from_alloc, to_alloc) = get_allocs(nodes.clone(), link);

        cr.move_to(
            (from_alloc.x + from_alloc.width).into(),
            (from_alloc.y + (from_alloc.height / 2)).into()
        );
        cr.line_to(
            to_alloc.x.into(),
            (to_alloc.y + (to_alloc.height / 2)).into()
        );

        cr.stroke();
    }
}

fn get_allocs(
    nodes: Rc<RefCell<HashMap<u32, PipewireNode>>>,
    link: &crate::PipewireLink,
) -> (gtk::Allocation, gtk::Allocation) {
    let from_alloc = &nodes
        .borrow()
        .get(&link.node_from)
        .unwrap()
        .get_outgoing_port(link.port_from)
        .unwrap()
        .get_allocation();
    let to_alloc = &nodes
        .borrow()
        .get(&link.node_to)
        .unwrap()
        .get_ingoing_port(link.port_to)
        .unwrap()
        .get_allocation();

    (from_alloc.to_owned(), to_alloc.to_owned())
}
