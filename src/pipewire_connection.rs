use gtk::glib::{self, clone};
use libspa::ForeignDict;
use log::trace;
use once_cell::unsync::OnceCell;
use pipewire as pw;
use pw::registry::GlobalObject;

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// This struct is responsible for communication with the pipewire server.
/// The owner of this struct can subscribe to notifications for globals added or removed.
///
/// It's `roundtrip` function must be called regularly to receive updates.
pub struct PipewireConnection {
    mainloop: pw::MainLoop,
    _context: pw::Context<pw::MainLoop>,
    core: pw::Core,
    registry: pw::registry::Registry,
    listeners: OnceCell<pw::registry::Listener>,
    on_global_add: Option<Box<dyn Fn(&GlobalObject<ForeignDict>)>>,
    on_global_remove: Option<Box<dyn Fn(u32)>>,
}

impl PipewireConnection {
    /// Create a new Pipewire Connection.
    ///
    /// This returns an `Rc`, because weak references to the result are needed inside closures set up during creation.
    pub fn new() -> Result<Rc<RefCell<Self>>, pw::Error> {
        // Initialize pipewire lib and obtain needed pipewire objects.
        pw::init();
        let mainloop = pw::MainLoop::new()?;
        let context = pw::Context::new(&mainloop)?;
        let core = context.connect(None)?;
        let registry = core.get_registry()?;

        let result = Rc::new(RefCell::new(Self {
            mainloop,
            _context: context,
            core,
            registry,
            listeners: OnceCell::new(),
            on_global_add: None,
            on_global_remove: None,
        }));

        // Notify state on globals added / removed
        let listeners = result
            .borrow()
            .registry
            .add_listener_local()
            .global(clone!(@weak result as this => move |global| {
                trace!("Global is added: {}", global.id);
                let con = this.borrow();
                if let Some(callback) = con.on_global_add.as_ref() {
                    callback(global)
                } else {
                    trace!("No on_global_add callback registered");
                }
            }))
            .global_remove(clone!(@weak result as this => move |id| {
                trace!("Global is removed: {}", id);
                let con = this.borrow();
                if let Some(callback) = con.on_global_remove.as_ref() {
                    callback(id)
                } else {
                    trace!("No on_global_remove callback registered");
                }
            }))
            .register();

        // Makeshift `expect()`: listeners does not implement `Debug`, so we can not use `expect`.
        assert!(
            result.borrow_mut().listeners.set(listeners).is_ok(),
            "PipewireConnection.listeners field already set"
        );

        Ok(result)
    }

    /// Receive all events from the pipewire server, sending them to the `pipewire_state` struct for processing.
    pub fn roundtrip(&self) {
        trace!("Starting roundtrip");

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

        trace!("Roundtrip finished");
    }

    /// Set or unset a callback that gets called when a new global is added.
    pub fn on_global_add(&mut self, callback: Option<Box<dyn Fn(&GlobalObject<ForeignDict>)>>) {
        self.on_global_add = callback;
    }

    /// Set or unset a callback that gets called when a global is removed.
    pub fn on_global_remove(&mut self, callback: Option<Box<dyn Fn(u32)>>) {
        self.on_global_remove = callback;
    }
}
