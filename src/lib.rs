//! fun60_analog_plugin — Wooting Analog SDK plugin for the MonsGeek FUN60 Ultra.
//!
//! Bridges the `iot_driver depth-server` Unix domain socket into the Wooting
//! Analog SDK's C-ABI plugin interface, exposing per-key analog depth values.

pub mod abi;
pub mod ffi;
pub mod hid;
pub mod keymap;
pub mod socket;
pub mod state;
