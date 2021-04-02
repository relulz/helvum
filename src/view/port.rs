use gtk::{glib, prelude::*, subclass::prelude::*};

use crate::controller::MediaType;

mod imp {
    use once_cell::unsync::OnceCell;

    use super::*;

    /// Graphical representation of a pipewire port.
    #[derive(Default)]
    pub struct Port {
        pub(super) id: OnceCell<u32>,
        pub(super) direction: OnceCell<pipewire::port::Direction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Port {
        const NAME: &'static str = "Port";
        type Type = super::Port;
        type ParentType = gtk::Button;
    }

    impl ObjectImpl for Port {}
    impl WidgetImpl for Port {}
    impl ButtonImpl for Port {}
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Button, gtk::Widget;
}

impl Port {
    pub fn new(
        id: u32,
        name: &str,
        direction: pipewire::port::Direction,
        media_type: Option<MediaType>,
    ) -> Self {
        // Create the widget and initialize needed fields
        let res: Self = glib::Object::new(&[]).expect("Failed to create Port");
        let private = imp::Port::from_instance(&res);
        private.id.set(id).expect("Port id already set");
        private
            .direction
            .set(direction)
            .expect("Port direction already set");

        res.set_child(Some(&gtk::Label::new(Some(name))));

        // Color the port according to its media type.
        match media_type {
            Some(MediaType::Video) => res.add_css_class("video"),
            Some(MediaType::Audio) => res.add_css_class("audio"),
            Some(MediaType::Midi) => res.add_css_class("midi"),
            None => {}
        }

        res
    }

    pub fn direction(&self) -> &pipewire::port::Direction {
        let private = imp::Port::from_instance(self);
        private.direction.get().expect("Port direction is not set")
    }
}
