mod snake_move;
pub use snake_move::*;

#[cfg(feature = "clib")]
mod c_exports;
