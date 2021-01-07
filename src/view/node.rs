use super::graph_view::GraphView;

use gtk::{glib, prelude::*, subclass::prelude::*, WidgetExt};
use pipewire::port::Direction;

use std::{collections::HashMap, rc::Rc};

mod imp {
    use super::*;

    use std::cell::{Cell, RefCell};

    pub struct Node {
        pub(super) grid: gtk::Grid,
        pub(super) label: gtk::Label,
        pub(super) ports: RefCell<HashMap<u32, Rc<crate::view::port::Port>>>,
        pub(super) num_ports_in: Cell<u32>,
        pub(super) num_ports_out: Cell<u32>,
    }

    impl ObjectSubclass for Node {
        const NAME: &'static str = "Node";
        type Type = super::Node;
        type ParentType = gtk::Frame;
        type Instance = glib::subclass::simple::InstanceStruct<Self>;
        type Class = glib::subclass::simple::ClassStruct<Self>;

        glib::object_subclass!();

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn new() -> Self {
            let grid = gtk::Grid::new();
            let label = gtk::Label::new(None);

            grid.attach(&label, 0, 0, 2, 1);

            let motion_controller = gtk::EventControllerMotion::new();
            motion_controller.connect_enter(|controller, _, _| {
                // Tell the graphview that the Node is the target of a drag when the mouse enters its label
                let widget = controller
                    .get_widget()
                    .expect("Controller with enter event has no widget")
                    .get_ancestor(super::Node::static_type())
                    .expect("Node label does not have a node ancestor widget");
                widget
                    .get_ancestor(GraphView::static_type())
                    .expect("Node with enter event is not on graph")
                    .dynamic_cast::<GraphView>()
                    .unwrap()
                    .set_dragged(Some(widget));
            });
            motion_controller.connect_leave(|controller| {
                // Tell the graphview that the Node is no longer the target of a drag when the mouse leaves.
                // FIXME: Check that we are the current target before setting none.
                controller
                    .get_widget()
                    .expect("Controller with leave event has no widget")
                    .get_ancestor(GraphView::static_type())
                    .expect("Node with leave event is not on graph")
                    .dynamic_cast::<GraphView>()
                    .unwrap()
                    .set_dragged(None);
            });
            label.add_controller(&motion_controller);

            // Display a grab cursor when the mouse is over the label so the user knows the node can be dragged.
            label.set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

            Self {
                grid,
                label,
                ports: RefCell::new(HashMap::new()),
                num_ports_in: Cell::new(0),
                num_ports_out: Cell::new(0),
            }
        }
    }

    impl ObjectImpl for Node {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            self.grid.set_parent(obj);
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.grid.unparent();
        }
    }

    impl FrameImpl for Node {}
    impl WidgetImpl for Node {}
}

glib::wrapper! {
    pub struct Node(ObjectSubclass<imp::Node>)
        @extends gtk::Widget;
}

impl Node {
    pub fn new(name: &str) -> Self {
        let res: Self = glib::Object::new(&[]).expect("Failed to create Node");
        let private = imp::Node::from_instance(&res);

        private.label.set_text(name);

        res
    }

    pub fn add_port(&mut self, id: u32, port: super::port::Port) {
        let private = imp::Node::from_instance(self);

        match port.direction {
            Direction::Input => {
                private
                    .grid
                    .attach(&port.widget, 0, private.num_ports_in.get() as i32 + 1, 1, 1);
                private.num_ports_in.set(private.num_ports_in.get() + 1);
            }
            Direction::Output => {
                private.grid.attach(
                    &port.widget,
                    1,
                    private.num_ports_out.get() as i32 + 1,
                    1,
                    1,
                );
                private.num_ports_out.set(private.num_ports_out.get() + 1);
            }
        }

        private.ports.borrow_mut().insert(id, Rc::new(port));
    }

    pub fn get_port(&self, id: u32) -> Option<Rc<super::port::Port>> {
        let private = imp::Node::from_instance(self);
        private
            .ports
            .borrow_mut()
            .get(&id)
            .map(|port_rc| port_rc.clone())
    }
}
