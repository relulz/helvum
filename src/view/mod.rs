//! The view presented to the user.
//!
//! This module contains gtk widgets needed to present the graphical user interface.

mod graph_view;
mod node;
mod port;

pub use graph_view::GraphView;
pub use node::Node;
pub use port::Port;
