/// Graphical representation of a pipewire port.
pub struct Port {
    pub(super) widget: gtk::Button,
    pub id: u32,
    pub direction: pipewire::port::Direction,
}

impl Port {
    pub fn new(id: u32, name: &str, direction: pipewire::port::Direction) -> Self {
        Self {
            widget: gtk::Button::with_label(name),
            id,
            direction,
        }
    }
}
