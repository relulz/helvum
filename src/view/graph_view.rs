use super::{Node, Port};

use gtk::{
    glib::{self, clone},
    graphene, gsk,
    prelude::*,
    subclass::prelude::*,
};
use log::{error, warn};

use std::collections::HashMap;

mod imp {
    use super::*;

    use std::{cell::RefCell, rc::Rc};

    use log::warn;

    #[derive(Default)]
    pub struct GraphView {
        pub(super) nodes: RefCell<HashMap<u32, Node>>,
        pub(super) links: RefCell<HashMap<u32, crate::PipewireLink>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphView {
        const NAME: &'static str = "GraphView";
        type Type = super::GraphView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.set_layout_manager_type::<gtk::FixedLayout>();
        }
    }

    impl ObjectImpl for GraphView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let drag_state = Rc::new(RefCell::new(None));
            let drag_controller = gtk::GestureDrag::new();

            drag_controller.connect_drag_begin(
                clone!(@strong drag_state => move |drag_controller, x, y| {
                let mut drag_state = drag_state.borrow_mut();
                let widget = drag_controller
                    .widget()
                    .expect("drag-begin event has no widget")
                    .dynamic_cast::<Self::Type>()
                    .expect("drag-begin event is not on the GraphView");
                // pick() should at least return the widget itself.
                let target = widget.pick(x, y, gtk::PickFlags::DEFAULT).expect("drag-begin pick() did not return a widget");
                *drag_state = if target.ancestor(Port::static_type()).is_some() {
                    // The user targeted a port, so the dragging should be handled by the Port
                    // component instead of here.
                    None
                } else if let Some(target) = target.ancestor(Node::static_type()) {
                    // The user targeted a Node without targeting a specific Port.
                    // Drag the Node around the screen.
                    let (x, y) = widget.get_node_position(&target);
                    Some((target, x, y))
                } else {
                    None
                }
            }));
            drag_controller.connect_drag_update(
                clone!(@strong drag_state => move |drag_controller, x, y| {
                    let widget = drag_controller
                        .widget()
                        .expect("drag-update event has no widget")
                        .dynamic_cast::<Self::Type>()
                        .expect("drag-update event is not on the GraphView");
                    let drag_state = drag_state.borrow();
                    if let Some((ref node, x1, y1)) = *drag_state {
                        widget.move_node(node, x1 + x as f32, y1 + y as f32);
                    }
                }
                ),
            );
            obj.add_controller(&drag_controller);
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.nodes
                .borrow()
                .values()
                .for_each(|node| node.unparent())
        }
    }

    impl WidgetImpl for GraphView {
        fn snapshot(&self, widget: &Self::Type, snapshot: &gtk::Snapshot) {
            /* FIXME: A lot of hardcoded values in here.
            Try to use relative units (em) and colours from the theme as much as possible. */

            let alloc = widget.allocation();

            let background_cr = snapshot
                .append_cairo(&graphene::Rect::new(
                    0.0,
                    0.0,
                    alloc.width as f32,
                    alloc.height as f32,
                ))
                .expect("Failed to get cairo context");

            // Try to replace the background color with a darker one from the theme.
            if let Some(rgba) = widget.style_context().lookup_color("text_view_bg") {
                background_cr.set_source_rgb(rgba.red.into(), rgba.green.into(), rgba.blue.into());
                if let Err(e) = background_cr.paint() {
                    warn!("Failed to paint graphview background: {}", e);
                };
            } // TODO: else log colour not found

            // Draw a nice grid on the background.
            background_cr.set_source_rgb(0.18, 0.18, 0.18);
            background_cr.set_line_width(0.2); // TODO: Set to 1px
            let mut y = 0.0;
            while y < alloc.height.into() {
                background_cr.move_to(0.0, y);
                background_cr.line_to(alloc.width.into(), y);
                y += 20.0; // TODO: Change to em;
            }
            let mut x = 0.0;
            while x < alloc.width.into() {
                background_cr.move_to(x, 0.0);
                background_cr.line_to(x, alloc.height.into());
                x += 20.0; // TODO: Change to em;
            }
            if let Err(e) = background_cr.stroke() {
                warn!("Failed to draw graphview grid: {}", e);
            };

            // Draw all children
            self.nodes
                .borrow()
                .values()
                .for_each(|node| self.instance().snapshot_child(node, snapshot));

            // Draw all links
            let link_cr = snapshot
                .append_cairo(&graphene::Rect::new(
                    0.0,
                    0.0,
                    alloc.width as f32,
                    alloc.height as f32,
                ))
                .expect("Failed to get cairo context");
            link_cr.set_line_width(2.0);
            link_cr.set_source_rgb(0.0, 0.0, 0.0);
            for link in self.links.borrow().values() {
                if let Some((from_x, from_y, to_x, to_y)) = self.get_link_coordinates(link) {
                    link_cr.move_to(from_x, from_y);

                    // Place curve control offset by half the x distance between the two points.
                    // This makes the curve scale well for varying distances between the two ports,
                    // especially when the output port is farther right than the input port.
                    let half_x_dist = f64::abs(from_x - to_x) / 2.0;
                    link_cr.curve_to(
                        from_x + half_x_dist,
                        from_y,
                        to_x - half_x_dist,
                        to_y,
                        to_x,
                        to_y,
                    );

                    if let Err(e) = link_cr.stroke() {
                        warn!("Failed to draw graphview links: {}", e);
                    };
                } else {
                    warn!("Could not get allocation of ports of link: {:?}", link);
                }
            }
        }
    }

    impl GraphView {
        /// Get coordinates for the drawn link to start at and to end at.
        ///
        /// # Returns
        /// `Some((from_x, from_y, to_x, to_y))` if all objects the links refers to exist as widgets.
        fn get_link_coordinates(&self, link: &crate::PipewireLink) -> Option<(f64, f64, f64, f64)> {
            let nodes = self.nodes.borrow();

            // For some reason, gtk4::WidgetExt::translate_coordinates gives me incorrect values,
            // so we manually calculate the needed offsets here.

            let from_port = &nodes.get(&link.node_from)?.get_port(link.port_from)?;
            let gtk::Allocation {
                x: mut fx,
                y: mut fy,
                width: fw,
                height: fh,
            } = from_port.allocation();
            let from_node = from_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let gtk::Allocation { x: fnx, y: fny, .. } = from_node.allocation();
            fx += fnx + fw;
            fy += fny + (fh / 2);

            let to_port = &nodes.get(&link.node_to)?.get_port(link.port_to)?;
            let gtk::Allocation {
                x: mut tx,
                y: mut ty,
                height: th,
                ..
            } = to_port.allocation();
            let to_node = to_port
                .ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let gtk::Allocation { x: tnx, y: tny, .. } = to_node.allocation();
            tx += tnx;
            ty += tny + (th / 2);

            Some((fx.into(), fy.into(), tx.into(), ty.into()))
        }
    }
}

glib::wrapper! {
    pub struct GraphView(ObjectSubclass<imp::GraphView>)
        @extends gtk::Widget;
}

impl GraphView {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create GraphView")
    }

    pub fn add_node(&self, id: u32, node: Node) {
        let private = imp::GraphView::from_instance(self);
        node.set_parent(self);

        // Place widgets in colums of 4, growing down, then right.
        // TODO: Make a better positioning algorithm.
        let x = (private.nodes.borrow().len() / 4) as f32 * 400.0; // This relies on integer division rounding down.
        let y = private.nodes.borrow().len() as f32 % 4.0 * 100.0;

        self.move_node(&node.clone().upcast(), x, y);

        private.nodes.borrow_mut().insert(id, node);
    }

    pub fn remove_node(&self, id: u32) {
        let private = imp::GraphView::from_instance(self);
        let mut nodes = private.nodes.borrow_mut();
        if let Some(node) = nodes.remove(&id) {
            node.unparent();
        } else {
            warn!("Tried to remove non-existant node (id={}) from graph", id);
        }
    }

    pub fn add_port(&self, node_id: u32, port_id: u32, port: crate::view::port::Port) {
        let private = imp::GraphView::from_instance(self);

        if let Some(node) = private.nodes.borrow_mut().get_mut(&node_id) {
            node.add_port(port_id, port);
        } else {
            error!(
                "Node with id {} not found when trying to add port with id {} to graph",
                node_id, port_id
            );
        }
    }

    pub fn remove_port(&self, id: u32, node_id: u32) {
        let private = imp::GraphView::from_instance(self);
        let nodes = private.nodes.borrow();
        if let Some(node) = nodes.get(&node_id) {
            node.remove_port(id);
        }
    }

    /// Add a link to the graph.
    ///
    /// `add_link` takes three arguments: `link_id` is the id of the link as assigned by the pipewire server,
    /// `from` and `to` are the id's of the ingoing and outgoing port, respectively.
    pub fn add_link(&self, link_id: u32, link: crate::PipewireLink) {
        let private = imp::GraphView::from_instance(self);
        private.links.borrow_mut().insert(link_id, link);
        self.queue_draw();
    }

    pub fn remove_link(&self, id: u32) {
        let private = imp::GraphView::from_instance(self);
        let mut links = private.links.borrow_mut();
        links.remove(&id);

        self.queue_draw();
    }

    pub(super) fn get_node_position(&self, node: &gtk::Widget) -> (f32, f32) {
        let layout_manager = self
            .layout_manager()
            .expect("Failed to get layout manager")
            .dynamic_cast::<gtk::FixedLayout>()
            .expect("Failed to cast to FixedLayout");

        let node = layout_manager
            .layout_child(node)
            .expect("Could not get layout child")
            .dynamic_cast::<gtk::FixedLayoutChild>()
            .expect("Could not cast to FixedLayoutChild");
        let transform = node.transform().unwrap_or_default();
        transform.to_translate()
    }

    pub(super) fn move_node(&self, node: &gtk::Widget, x: f32, y: f32) {
        let layout_manager = self
            .layout_manager()
            .expect("Failed to get layout manager")
            .dynamic_cast::<gtk::FixedLayout>()
            .expect("Failed to cast to FixedLayout");

        let transform = gsk::Transform::new()
            // Nodes should not be able to be dragged out of the view, so we use `max(coordinate, 0.0)` to prevent that.
            .translate(&graphene::Point::new(f32::max(x, 0.0), f32::max(y, 0.0)))
            .unwrap();

        layout_manager
            .layout_child(node)
            .expect("Could not get layout child")
            .dynamic_cast::<gtk::FixedLayoutChild>()
            .expect("Could not cast to FixedLayoutChild")
            .set_transform(&transform);

        // FIXME: If links become proper widgets,
        // we don't need to redraw the full graph everytime.
        self.queue_draw();
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}
