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
pub mod debug;
pub mod collections;
pub mod string;
pub mod ref_type;
pub mod transaction;
pub mod agent;
pub mod thread_pool;
pub mod channel;
pub mod go_block;

// Re-export main types
pub use value::{Value, ValueTag};
pub use list::PersistentList;
pub use vector::PersistentVector;
pub use map::ClorusHashMap;
pub use set::ClorusHashSet;
pub use atom::ClorusAtom;
