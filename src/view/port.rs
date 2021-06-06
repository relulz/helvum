use gtk::{
    gdk,
    glib::{self, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use log::warn;
use pipewire::spa::Direction;

use crate::MediaType;

mod imp {
    use once_cell::{sync::Lazy, unsync::OnceCell};
    use pipewire::spa::Direction;

    use super::*;

    /// Graphical representation of a pipewire port.
    #[derive(Default)]
    pub struct Port {
        pub(super) id: OnceCell<u32>,
        pub(super) direction: OnceCell<Direction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Port {
        const NAME: &'static str = "Port";
        type Type = super::Port;
        type ParentType = gtk::Button;
    }

    impl ObjectImpl for Port {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder(
                    "port-toggled",
                    // Provide id of output port and input port to signal handler.
                    &[<u32>::static_type().into(), <u32>::static_type().into()],
                    // signal handler sends back nothing.
                    <()>::static_type().into(),
                )
                .build()]
            });

            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for Port {}
    impl ButtonImpl for Port {}
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Button, gtk::Widget;
}

impl Port {
    pub fn new(id: u32, name: &str, direction: Direction, media_type: Option<MediaType>) -> Self {
        // Create the widget and initialize needed fields
        let res: Self = glib::Object::new(&[]).expect("Failed to create Port");
        let private = imp::Port::from_instance(&res);
        private.id.set(id).expect("Port id already set");
        private
            .direction
            .set(direction)
            .expect("Port direction already set");

        res.set_child(Some(&gtk::Label::new(Some(name))));

        // Add either a drag source or drop target controller depending on direction,
        // they will be responsible for link creation by dragging an output port onto an input port.
        //
        // FIXME: The type used for dragging is simply a u32.
        //   This means that anything that provides a u32 could be dragged onto a input port,
        //   leading to that port trying to create a link to an invalid output port.
        //   We should use a newtype instead of a plain u32.
        //   Additionally, this does not protect against e.g. dropping an outgoing audio port on an ingoing video port.
        match direction {
            Direction::Input => {
                let drop_target = gtk::DropTarget::new(u32::static_type(), gdk::DragAction::COPY);
                let this = res.clone();
                drop_target.connect_drop(move |drop_target, val, _, _| {
                    if let Ok(source_id) = val.get::<u32>() {
                        // Get the callback registered in the widget and call it
                        drop_target
                            .widget()
                            .expect("Drop target has no widget")
                            .emit_by_name("port-toggled", &[&source_id, &this.id()])
                            .expect("Failed to send signal");
                    } else {
                        warn!("Invalid type dropped on ingoing port");
                    }

                    true
                });
                res.add_controller(&drop_target);
            }
            Direction::Output => {
                // The port will simply provide its pipewire id to the drag target.
                let drag_src = gtk::DragSourceBuilder::new()
                    .content(&gdk::ContentProvider::for_value(&(id.to_value())))
                    .build();
                res.add_controller(&drag_src);

                // Display a grab cursor when the mouse is over the port so the user knows it can be dragged to another port.
                res.set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());
            }
        }

        // Color the port according to its media type.
        match media_type {
            Some(MediaType::Video) => res.add_css_class("video"),
            Some(MediaType::Audio) => res.add_css_class("audio"),
            Some(MediaType::Midi) => res.add_css_class("midi"),
            None => {}
        }

        res
    }

    pub fn id(&self) -> u32 {
        let private = imp::Port::from_instance(self);
        private.id.get().copied().expect("Port id is not set")
    }

    pub fn direction(&self) -> &Direction {
        let private = imp::Port::from_instance(self);
        private.direction.get().expect("Port direction is not set")
    }
}
