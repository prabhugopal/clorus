use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use crate::set::{release_set, ClorusHashSet};
use crate::value::{clorus_equals, clorus_release, Value, ValueTag};

type Hierarchy = HashMap<String, HashSet<String>>;

fn hierarchy_store() -> &'static Mutex<Hierarchy> {
    static STORE: OnceLock<Mutex<Hierarchy>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn value_key(val: *mut Value) -> Option<String> {
    if val.is_null() {
        return Some("nil".to_string());
    }

    match (*val).tag() {
        ValueTag::Keyword => Some(format!(":{}", (*val).as_keyword())),
        ValueTag::Symbol => Some(format!("sym:{}", (*val).as_string())),
        ValueTag::String => Some(format!("str:{}", (*val).as_string())),
        ValueTag::Long => Some(format!("long:{}", (*val).as_long())),
        ValueTag::Double => Some(format!("double:{:.17}", (*val).as_double())),
        ValueTag::Bool => Some(format!("bool:{}", if (*val).as_bool() { "true" } else { "false" })),
        ValueTag::Nil => Some("nil".to_string()),
        _ => None,
    }
}

fn isa_key(store: &Hierarchy, child: &str, parent: &str) -> bool {
    if child == parent {
        return true;
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut stack = vec![child.to_string()];

    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }

        if let Some(parents) = store.get(&current) {
            for p in parents {
                if p == parent {
                    return true;
                }
                stack.push(p.clone());
            }
        }
    }

    false
}

unsafe fn key_to_value(key: &str) -> Option<*mut Value> {
    if key == "nil" {
        return Some(Value::nil());
    }

    if let Some(rest) = key.strip_prefix(':') {
        return Some(Value::keyword(rest));
    }
    if let Some(rest) = key.strip_prefix("str:") {
        return Some(Value::string(rest));
    }
    if let Some(rest) = key.strip_prefix("long:") {
        if let Ok(v) = rest.parse::<i64>() {
            return Some(Value::long(v));
        }
    }
    if let Some(rest) = key.strip_prefix("double:") {
        if let Ok(v) = rest.parse::<f64>() {
            return Some(Value::double(v));
        }
    }
    if let Some(rest) = key.strip_prefix("bool:") {
        return Some(Value::boolean(rest == "true"));
    }
    if let Some(rest) = key.strip_prefix("sym:") {
        // Symbol runtime tagging is still evolving; preserve data as string key.
        return Some(Value::string(rest));
    }

    None
}

unsafe fn keys_to_set<I>(keys: I) -> *mut Value
where
    I: IntoIterator<Item = String>,
{
    let mut set_ptr = ClorusHashSet::empty();
    for key in keys.into_iter() {
        if let Some(v) = key_to_value(&key) {
            let new_set = ClorusHashSet::conj(set_ptr, v);
            release_set(set_ptr);
            set_ptr = new_set;
            clorus_release(v);
        }
    }
    Value::from_ptr(ValueTag::HashSet, set_ptr as *mut u8)
}

#[no_mangle]
pub extern "C" fn clorus_derive(child: *mut Value, parent: *mut Value) -> *mut Value {
    unsafe {
        if child.is_null() || parent.is_null() {
            return child;
        }

        let Some(child_key) = value_key(child) else { return child; };
        let Some(parent_key) = value_key(parent) else { return child; };

        let mut store = hierarchy_store().lock().expect("hierarchy mutex poisoned");
        if child_key != parent_key {
            if isa_key(&store, &parent_key, &child_key) {
                // Prevent cycles: ignore derive that would make parent already descend from child.
                return child;
            }
            store.entry(child_key).or_default().insert(parent_key);
        }

        child
    }
}

#[no_mangle]
pub extern "C" fn clorus_underive(child: *mut Value, parent: *mut Value) -> *mut Value {
    unsafe {
        if child.is_null() || parent.is_null() {
            return child;
        }

        let Some(child_key) = value_key(child) else { return child; };
        let Some(parent_key) = value_key(parent) else { return child; };

        let mut store = hierarchy_store().lock().expect("hierarchy mutex poisoned");
        if let Some(parents) = store.get_mut(&child_key) {
            parents.remove(&parent_key);
            if parents.is_empty() {
                store.remove(&child_key);
            }
        }

        child
    }
}

#[no_mangle]
pub extern "C" fn clorus_isa(child: *mut Value, parent: *mut Value) -> bool {
    unsafe {
        if child.is_null() || parent.is_null() {
            return child.is_null() && parent.is_null();
        }

        if clorus_equals(child, parent) {
            return true;
        }

        let Some(child_key) = value_key(child) else { return false; };
        let Some(parent_key) = value_key(parent) else { return false; };
        let store = hierarchy_store().lock().expect("hierarchy mutex poisoned");
        isa_key(&store, &child_key, &parent_key)
    }
}

#[no_mangle]
pub extern "C" fn clorus_isa_i32(child: *mut Value, parent: *mut Value) -> i32 {
    if clorus_isa(child, parent) { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn clorus_parents(child: *mut Value) -> *mut Value {
    unsafe {
        let Some(child_key) = value_key(child) else { return Value::nil(); };
        let store = hierarchy_store().lock().expect("hierarchy mutex poisoned");
        let parents: Vec<String> = store.get(&child_key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        keys_to_set(parents)
    }
}

#[no_mangle]
pub extern "C" fn clorus_ancestors(child: *mut Value) -> *mut Value {
    unsafe {
        let Some(child_key) = value_key(child) else { return Value::nil(); };
        let store = hierarchy_store().lock().expect("hierarchy mutex poisoned");

        let mut visited: HashSet<String> = HashSet::new();
        let mut stack = vec![child_key];
        while let Some(current) = stack.pop() {
            if let Some(parents) = store.get(&current) {
                for p in parents {
                    if visited.insert(p.clone()) {
                        stack.push(p.clone());
                    }
                }
            }
        }

        keys_to_set(visited.into_iter().collect::<Vec<_>>())
    }
}

#[no_mangle]
pub extern "C" fn clorus_descendants(parent: *mut Value) -> *mut Value {
    unsafe {
        let Some(parent_key) = value_key(parent) else { return Value::nil(); };
        let store = hierarchy_store().lock().expect("hierarchy mutex poisoned");

        let mut nodes: HashSet<String> = HashSet::new();
        for (child, parents) in store.iter() {
            nodes.insert(child.clone());
            for p in parents {
                nodes.insert(p.clone());
            }
        }

        let mut descendants: Vec<String> = Vec::new();
        for node in nodes.into_iter() {
            if node != parent_key && isa_key(&store, &node, &parent_key) {
                descendants.push(node);
            }
        }

        keys_to_set(descendants)
    }
}
