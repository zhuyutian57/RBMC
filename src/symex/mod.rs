
mod exec_state;
mod frame;
mod namespace;
mod place_state;
mod projection;
mod renaming;
mod state;
mod value_set;

pub mod symex;
pub(super) mod symex_assign;
pub(super) mod symex_branch;
pub(super) mod symex_drop;
pub(super) mod symex_dealloc;
pub(super) mod symex_function;
pub(super) mod symex_util;
pub(super) mod symex_memory;
pub(super) mod symex_return;