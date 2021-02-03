use super::Node;

use gtk::{glib, graphene, gsk, prelude::*, subclass::prelude::*, WidgetExt};

use std::collections::HashMap;

mod imp {
    use super::*;

    use gtk::{gdk, WidgetExt};

    use std::{cell::RefCell, rc::Rc};

    pub struct GraphView {
        pub(super) nodes: RefCell<HashMap<u32, Node>>,
        pub(super) links: RefCell<HashMap<u32, crate::PipewireLink>>,
        pub(super) dragged: Rc<RefCell<Option<gtk::Widget>>>,
    }

    impl ObjectSubclass for GraphView {
        const NAME: &'static str = "GraphView";
        type Type = super::GraphView;
        type ParentType = gtk::Widget;
        type Instance = glib::subclass::simple::InstanceStruct<Self>;
        type Class = glib::subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn class_init(klass: &mut Self::Class) {
            // The layout manager determines how child widgets are laid out.
            klass.set_layout_manager_type::<gtk::FixedLayout>();
        }

        fn new() -> Self {
            Self {
                nodes: RefCell::new(HashMap::new()),
                links: RefCell::new(HashMap::new()),
                dragged: Rc::new(RefCell::new(None)),
            }
        }
    }

    impl ObjectImpl for GraphView {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Move the Node that is currently being dragged to the cursor position as long as Mouse Button 1 is held.
            let motion_controller = gtk::EventControllerMotion::new();
            motion_controller.connect_motion(|controller, x, y| {
                let instance = controller
                    .get_widget()
                    .unwrap()
                    .dynamic_cast::<Self::Type>()
                    .unwrap();
                let this = imp::GraphView::from_instance(&instance);

                if let Some(ref widget) = *this.dragged.borrow() {
                    if controller
                        .get_current_event()
                        .unwrap()
                        .get_modifier_state()
                        .contains(gdk::ModifierType::BUTTON1_MASK)
                    {
                        instance.move_node(&widget, x as f32, y as f32);
                    }
                };
            });
            obj.add_controller(&motion_controller);
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

            let alloc = widget.get_allocation();

            let cr = snapshot
                .append_cairo(&graphene::Rect::new(
                    0.0,
                    0.0,
                    alloc.width as f32,
                    alloc.height as f32,
                ))
                .expect("Failed to get cairo context");

            // Try to replace the background color with a darker one from the theme.
            if let Some(rgba) = widget.get_style_context().lookup_color("text_view_bg") {
                cr.set_source_rgb(rgba.red.into(), rgba.green.into(), rgba.blue.into());
                cr.paint();
            } // TODO: else log colour not found

            // Draw a nice grid on the background.
            cr.set_source_rgb(0.18, 0.18, 0.18);
            cr.set_line_width(0.2); // TODO: Set to 1px
            let mut y = 0.0;
            while y < alloc.height.into() {
                cr.move_to(0.0, y);
                cr.line_to(alloc.width as f64, y);
                y += 20.0; // TODO: Change to em;
            }
            let mut x = 0.0;
            while x < alloc.width as f64 {
                cr.move_to(x, 0.0);
                cr.line_to(x, alloc.height as f64);
                x += 20.0; // TODO: Change to em;
            }
            cr.stroke();

            // Draw all links
            cr.set_line_width(2.0);
            cr.set_source_rgb(0.0, 0.0, 0.0);
            for link in self.links.borrow().values() {
                if let Some((from_x, from_y, to_x, to_y)) = self.get_link_coordinates(link) {
                    cr.move_to(from_x, from_y);
                    cr.curve_to(from_x + 75.0, from_y, to_x - 75.0, to_y, to_x, to_y);
                    cr.stroke();
                } else {
                    log::warn!("Could not get allocation of ports of link: {:?}", link);
                }
            }

            // Draw all children
            self.nodes
                .borrow()
                .values()
                .for_each(|node| self.get_instance().snapshot_child(node, snapshot));
        }
    }

    impl GraphView {
        /// Get coordinates for the drawn link to start at and to end at.
        ///
        /// # Returns
        /// Some((from_x, from_y, to_x, to_y)) if all objects the links refers to exist as widgets.
        fn get_link_coordinates(&self, link: &crate::PipewireLink) -> Option<(f64, f64, f64, f64)> {
            let nodes = self.nodes.borrow();

            // For some reason, gtk4::WidgetExt::translate_coordinates gives me incorrect values,
            // so we manually calculate the needed offsets here.

            let from_port = &nodes.get(&link.node_from)?.get_port(link.port_from)?.widget;
            let gtk::Allocation {
                x: mut fx,
                y: mut fy,
                width: fw,
                height: fh,
            } = from_port.get_allocation();
            let from_node = from_port
                .get_ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let gtk::Allocation { x: fnx, y: fny, .. } = from_node.get_allocation();
            fx += fnx + fw;
            fy += fny + (fh / 2);

            let to_port = &nodes.get(&link.node_to)?.get_port(link.port_to)?.widget;
            let gtk::Allocation {
                x: mut tx,
                y: mut ty,
                height: th,
                ..
            } = to_port.get_allocation();
            let to_node = to_port
                .get_ancestor(Node::static_type())
                .expect("Port is not a child of a node");
            let gtk::Allocation { x: tnx, y: tny, .. } = to_node.get_allocation();
            tx += tnx;
            ty += tny + (th / 2);

            Some((fx as f64, fy as f64, tx as f64, ty as f64))
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
        }
    }

    pub fn add_port_to_node(&self, node_id: u32, port_id: u32, port: crate::view::port::Port) {
        let private = imp::GraphView::from_instance(self);

        if let Some(node) = private.nodes.borrow_mut().get_mut(&node_id) {
            node.add_port(port_id, port);
        } else {
            // FIXME: Log this instead
            log::error!(
                "Node with id {} not found when trying to add port with id {} to graph",
                node_id,
                port_id
            );
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

    pub fn set_dragged(&self, widget: Option<gtk::Widget>) {
        *imp::GraphView::from_instance(self).dragged.borrow_mut() = widget;
    }

    pub fn move_node(&self, node: &gtk::Widget, x: f32, y: f32) {
        let layout_manager = self
            .get_layout_manager()
            .expect("Failed to get layout manager")
            .dynamic_cast::<gtk::FixedLayout>()
            .expect("Failed to cast to FixedLayout");

        let transform = gsk::Transform::new()
            // Nodes should not be able to be dragged out of the view, so we use `max(coordinate, 0.0)` to prevent that.
            .translate(&graphene::Point::new(f32::max(x, 0.0), f32::max(y, 0.0)))
            .unwrap();

        layout_manager
            .get_layout_child(node)
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
