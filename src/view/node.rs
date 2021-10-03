// node.rs
//
// Copyright 2021 Tom A. Wagner <tom.a.wagner@protonmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-only

use gtk::{glib, prelude::*, subclass::prelude::*};
use pipewire::spa::Direction;

use std::collections::HashMap;

mod imp {
    use super::*;

    use std::cell::{Cell, RefCell};

    pub struct Node {
        pub(super) grid: gtk::Grid,
        pub(super) label: gtk::Label,
        pub(super) ports: RefCell<HashMap<u32, crate::view::port::Port>>,
        pub(super) num_ports_in: Cell<i32>,
        pub(super) num_ports_out: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Node {
        const NAME: &'static str = "Node";
        type Type = super::Node;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn new() -> Self {
            let grid = gtk::Grid::new();
            let label = gtk::Label::new(None);

            grid.attach(&label, 0, 0, 2, 1);

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

        match port.direction() {
            Direction::Input => {
                private
                    .grid
                    .attach(&port, 0, private.num_ports_in.get() + 1, 1, 1);
                private.num_ports_in.set(private.num_ports_in.get() + 1);
            }
            Direction::Output => {
                private
                    .grid
                    .attach(&port, 1, private.num_ports_out.get() + 1, 1, 1);
                private.num_ports_out.set(private.num_ports_out.get() + 1);
            }
        }

        private.ports.borrow_mut().insert(id, port);
    }

    pub fn get_port(&self, id: u32) -> Option<super::port::Port> {
        let private = imp::Node::from_instance(self);
        private.ports.borrow_mut().get(&id).cloned()
    }

    pub fn remove_port(&self, id: u32) {
        let private = imp::Node::from_instance(self);
        if let Some(port) = private.ports.borrow_mut().remove(&id) {
            match port.direction() {
                Direction::Input => private.num_ports_in.set(private.num_ports_in.get() - 1),
                Direction::Output => private.num_ports_in.set(private.num_ports_out.get() - 1),
            }

            port.unparent();
        }
    }
}
