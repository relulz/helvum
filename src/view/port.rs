use gtk::WidgetExt;

use crate::controller::MediaType;

/// Graphical representation of a pipewire port.
pub struct Port {
    pub widget: gtk::Button,
    pub id: u32,
    pub direction: pipewire::port::Direction,
}

impl Port {
    pub fn new(
        id: u32,
        name: &str,
        direction: pipewire::port::Direction,
        media_type: Option<MediaType>,
    ) -> Self {
        let widget = gtk::Button::with_label(name);

        // Color the port according to its media type.
        match media_type {
            Some(MediaType::Video) => widget.add_css_class("video"),
            Some(MediaType::Audio) => widget.add_css_class("audio"),
            Some(MediaType::Midi) => widget.add_css_class("midi"),
            None => {}
        }

        Self {
            widget,
            id,
            direction,
        }
    }
}
