use gtk::{
    gdk,
    glib::{self, clone, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use log::{trace, warn};
use pipewire::spa::Direction;

use crate::MediaType;

/// A helper struct for linking a output port to an input port.
/// It carries the output ports id.
#[derive(Clone, Debug, glib::GBoxed)]
#[gboxed(type_name = "HelvumForwardLink")]
struct ForwardLink(u32);

/// A helper struct for linking an input to an output port.
/// It carries the input ports id.
#[derive(Clone, Debug, glib::GBoxed)]
#[gboxed(type_name = "HelvumReversedLink")]
struct ReversedLink(u32);

mod imp {
    use once_cell::{sync::Lazy, unsync::OnceCell};
    use pipewire::spa::Direction;

    use super::*;

    /// Graphical representation of a pipewire port.
    #[derive(Default)]
    pub struct Port {
        pub(super) label: OnceCell<gtk::Label>,
        pub(super) id: OnceCell<u32>,
        pub(super) direction: OnceCell<Direction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Port {
        const NAME: &'static str = "Port";
        type Type = super::Port;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();

            // Make it look like a GTK button.
            klass.set_css_name("button");
        }
    }

    impl ObjectImpl for Port {
        fn dispose(&self, _obj: &Self::Type) {
            if let Some(label) = self.label.get() {
                label.unparent()
            }
        }

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
}

glib::wrapper! {
    pub struct Port(ObjectSubclass<imp::Port>)
        @extends gtk::Widget;
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

        let label = gtk::Label::new(Some(name));
        label.set_parent(&res);
        private
            .label
            .set(label)
            .expect("Port label was already set");

        // Add a drag source and drop target controller with the type depending on direction,
        // they will be responsible for link creation by dragging an output port onto an input port or the other way around.

        // FIXME: We should protect against different media types, e.g. it should not be possible to drop a video port on an audio port.

        // The port will simply provide its pipewire id to the drag target.
        let drag_src = gtk::DragSourceBuilder::new()
            .content(&gdk::ContentProvider::for_value(&match direction {
                Direction::Input => ReversedLink(id).to_value(),
                Direction::Output => ForwardLink(id).to_value(),
            }))
            .build();
        drag_src.connect_drag_begin(move |_, _| {
            trace!("Drag started from port {}", id);
        });
        drag_src.connect_drag_cancel(move |_, _, _| {
            trace!("Drag from port {} was cancelled", id);
            false
        });
        res.add_controller(&drag_src);

        // The drop target will accept either a `ForwardLink` or `ReversedLink` depending in its own direction,
        // and use it to emit its `port-toggled` signal.
        let drop_target = gtk::DropTarget::new(
            match direction {
                Direction::Input => ForwardLink::static_type(),
                Direction::Output => ReversedLink::static_type(),
            },
            gdk::DragAction::COPY,
        );
        match direction {
            Direction::Input => {
                drop_target.connect_drop(
                    clone!(@weak res as this => @default-panic, move |drop_target, val, _, _| {
                        if let Ok(ForwardLink(source_id)) = val.get::<ForwardLink>() {
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
                    }),
                );
            }
            Direction::Output => {
                drop_target.connect_drop(
                    clone!(@weak res as this => @default-panic, move |drop_target, val, _, _| {
                        if let Ok(ReversedLink(target_id)) = val.get::<ReversedLink>() {
                            // Get the callback registered in the widget and call it
                            drop_target
                                .widget()
                                .expect("Drop target has no widget")
                                .emit_by_name("port-toggled", &[&this.id(), &target_id])
                                .expect("Failed to send signal");
                        } else {
                            warn!("Invalid type dropped on outgoing port");
                        }

                        true
                    }),
                );
            }
        }
        res.add_controller(&drop_target);

        // Display a grab cursor when the mouse is over the port so the user knows it can be dragged to another port.
        res.set_cursor(gtk::gdk::Cursor::from_name("grab", None).as_ref());

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
