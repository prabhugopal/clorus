/// Clorus Core - Clojure-style convenience functions
///
/// This module provides simple, Clojure-style functions that are
/// easier to use than the low-level rust.fs API.
///
/// Examples:
/// - `slurp` instead of `fs/read-to-string`
/// - `spit` instead of `fs/write`

pub mod io;
pub mod string_utils;

// Re-export main functions
pub use io::{clorus_slurp, clorus_spit};
pub use string_utils::{
    clorus_string_count_newlines,
    clorus_string_find_nth_newline,
    clorus_string_get_line,
    clorus_string_split_lines,
    clorus_string_is_newline_at,
};

// Re-export runtime functions for linking
pub use clorus_runtime::map::{
    clorus_map_empty,
    clorus_map_assoc,
    clorus_map_get,
    clorus_map_count,
};
pub use clorus_runtime::vector::{
    clorus_vector_empty,
    clorus_vector_conj,
    clorus_vector_nth,
    clorus_vector_count,
};
pub use clorus_runtime::list::{
    clorus_list_empty,
    clorus_list_cons,
};
pub use clorus_runtime::keyword::{
    clorus_keyword,
    clorus_is_keyword,
    clorus_keyword_name,
};
pub use clorus_runtime::value::{
    clorus_value_long,
    clorus_value_double,
    clorus_value_as_long,
    clorus_value_as_double,
    clorus_value_string,
    clorus_value_as_cstring,
    clorus_retain,
    clorus_release,
    clorus_free_cstring,
};
