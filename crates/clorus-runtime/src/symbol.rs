/// Symbol support for Clorus
/// Symbols are interned named identifiers (without leading `:`).

use crate::value::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// Global symbol intern table.
/// Symbols are interned for pointer-stable identity.
/// Store as usize to keep the table Send-safe behind Mutex.
static SYMBOL_TABLE: Mutex<Option<HashMap<String, usize>>> = Mutex::new(None);

fn get_symbol_table() -> std::sync::MutexGuard<'static, Option<HashMap<String, usize>>> {
    let mut table = SYMBOL_TABLE.lock().unwrap();
    if table.is_none() {
        *table = Some(HashMap::new());
    }
    table
}

/// Intern a symbol - returns the same pointer for the same symbol string.
pub fn intern_symbol(name: &str) -> *mut Value {
    let mut table_guard = get_symbol_table();
    let table = table_guard.as_mut().unwrap();

    if let Some(&existing) = table.get(name) {
        existing as *mut Value
    } else {
        let symbol_val = Value::symbol(name);
        table.insert(name.to_string(), symbol_val as usize);
        symbol_val
    }
}
