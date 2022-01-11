// main.rs
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

mod application;
mod pipewire_connection;
mod view;

use glib::PRIORITY_DEFAULT;
use gtk::prelude::*;
use pipewire::spa::Direction;

/// Messages sent by the GTK thread to notify the pipewire thread.
#[derive(Debug, Clone)]
enum GtkMessage {
    /// Toggle a link between the two specified ports.
    ToggleLink { port_from: u32, port_to: u32 },
    /// Quit the event loop and let the thread finish.
    Terminate,
}

/// Messages sent by the pipewire thread to notify the GTK thread.
#[derive(Debug, Clone)]
enum PipewireMessage {
    NodeAdded {
        id: u32,
        name: String,
        node_type: Option<NodeType>,
    },
    PortAdded {
        id: u32,
        node_id: u32,
        name: String,
        direction: Direction,
        media_type: Option<MediaType>,
    },
    LinkAdded {
        id: u32,
        node_from: u32,
        port_from: u32,
        node_to: u32,
        port_to: u32,
        active: bool,
    },
    LinkStateChanged {
        id: u32,
        active: bool,
    },
    NodeRemoved {
        id: u32,
    },
    PortRemoved {
        id: u32,
        node_id: u32,
    },
    LinkRemoved {
        id: u32,
    },
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Input,
    Output,
}

#[derive(Debug, Copy, Clone)]
pub enum MediaType {
    Audio,
    Video,
    Midi,
}

#[derive(Debug, Clone)]
pub struct PipewireLink {
    pub node_from: u32,
    pub port_from: u32,
    pub node_to: u32,
    pub port_to: u32,
}

static GLIB_LOGGER: glib::GlibLogger = glib::GlibLogger::new(
    glib::GlibLoggerFormat::Structured,
    glib::GlibLoggerDomain::CrateTarget,
);

fn init_glib_logger() {
    log::set_logger(&GLIB_LOGGER).expect("Failed to set logger");

    // Glib does not have a "Trace" log level, so only print messages "Debug" or higher priority.
    log::set_max_level(log::LevelFilter::Debug);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_glib_logger();
    gtk::init()?;

    // Aquire main context so that we can attach the gtk channel later.
    let ctx = glib::MainContext::default();
    let _guard = ctx.acquire().unwrap();

    // Start the pipewire thread with channels in both directions.
    let (gtk_sender, gtk_receiver) = glib::MainContext::channel(PRIORITY_DEFAULT);
    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let pw_thread =
        std::thread::spawn(move || pipewire_connection::thread_main(gtk_sender, pw_receiver));

    let app = application::Application::new(gtk_receiver, pw_sender.clone());

    app.run();

    pw_sender
        .send(GtkMessage::Terminate)
        .expect("Failed to send message");

    pw_thread.join().expect("Pipewire thread panicked");

    Ok(())
}
