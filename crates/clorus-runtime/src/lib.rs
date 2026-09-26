/// Clorus Runtime Library
///
/// Provides persistent data structures with reference counting for the Clorus language.
/// Designed to be called from LLVM-generated code.

pub mod value;
pub mod list;
pub mod vector;
pub mod map;
pub mod set;
pub mod atom;
pub mod keyword;
pub mod symbol;
pub mod debug;
pub mod collections;
pub mod string;
pub mod ref_type;
pub mod transaction;
pub mod agent;
pub mod thread_pool;
pub mod channel;
pub mod go_block;
pub mod function;
pub mod transducer;
pub mod io;
pub mod var;  // Var support for dynamic bindings
pub mod arithmetic;  // Type-aware arithmetic operations
pub mod protocols;  // Protocol dispatch for deftype/defrecord
pub mod hash;  // Structural hashing helpers
pub mod hierarchy;  // derive/underive/isa? hierarchy relations
pub mod net;  // TCP socket primitives
pub mod keep_alive;  // Retains every extern "C" fn for JIT hosts -- see build.rs

// Re-export main types
pub use value::{Value, ValueTag};
pub use list::PersistentList;
pub use vector::PersistentVector;
pub use map::ClorusHashMap;
pub use set::ClorusHashSet;
pub use atom::ClorusAtom;

/// Exported wrapper for 3-arity get-in semantics.
#[no_mangle]
pub extern "C" fn clorus_map_get_in_or(
    map_val: *mut Value,
    keys: *mut Value,
    not_found: *mut Value,
) -> *mut Value {
    collections::map_ops::clorus_map_get_in_or(map_val, keys, not_found)
}
