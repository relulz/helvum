use crate::pipewire_state::PipewireState;

use gtk::glib::{self, clone};
use pipewire as pw;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// This struct is responsible for communication with the pipewire server.
/// It handles new globals appearing as well as globals being removed.
///
/// It's `roundtrip` function must be called regularly to receive updates.
pub struct PipewireConnection {
    mainloop: pw::MainLoop,
    _context: pw::Context<pw::MainLoop>,
    core: Rc<pw::Core>,
    _registry: pw::registry::Registry,
    _listeners: pw::registry::Listener,
    _state: Rc<RefCell<PipewireState>>,
}

impl PipewireConnection {
    pub fn new(state: PipewireState) -> Result<Self, String> {
        // Initialize pipewire lib and obtain needed pipewire objects.
        pw::init();
        let mainloop = pw::MainLoop::new().map_err(|_| "Failed to create pipewire mainloop!")?;
        let context =
            pw::Context::new(&mainloop).map_err(|_| "Failed to create pipewire context")?;
        let core = Rc::new(
            context
                .connect(None)
                .map_err(|_| "Failed to connect to pipewire core")?,
        );
        let registry = core
            .get_registry()
            .map_err(|_| "Failed to get pipewire registry")?;

        let state = Rc::new(RefCell::new(state));

        // Notify state on globals added / removed
        let _listeners = registry
            .add_listener_local()
            .global(clone!(@weak state => @default-panic, move |global| {
                state.borrow_mut().global(global);
            }))
            .global_remove(clone!(@weak state => @default-panic, move |id| {
                state.borrow_mut().global_remove(id);
            }))
            .register();

        Ok(Self {
            mainloop,
            _context: context,
            core,
            _registry: registry,
            _listeners,
            _state: state,
        })
    }

    /// Receive all events from the pipewire server, sending them to the `pipewire_state` struct for processing.
    pub fn roundtrip(&self) {
        let done = Rc::new(Cell::new(false));
        let pending = self
            .core
            .sync(0)
            .expect("Failed to trigger core sync event");

        let done_clone = done.clone();
        let loop_clone = self.mainloop.clone();

        let _listener = self
            .core
            .add_listener_local()
            .done(move |id, seq| {
                if id == pw::PW_ID_CORE && seq == pending {
                    done_clone.set(true);
                    loop_clone.quit();
                }
            })
            .register();

        while !done.get() {
            self.mainloop.run();
        }
    }
}
