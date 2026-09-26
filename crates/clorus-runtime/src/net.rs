/// TCP socket primitives for Clorus.
///
/// This is the "lang" tier of networking (mirroring Go's split between
/// `net`, which is deeply wired into the runtime, and `net/http`, which is
/// pure Go on top of it): raw listen/accept/read/write/close, implemented
/// as thin wrappers over Rust's own `std::net` -- not a reimplementation of
/// TCP, just making it a first-class Clorus builtin rather than something a
/// user has to opt into via a separate Rust dependency. HTTP itself belongs
/// in stdlib Clorus source on top of these, the same way `net/http` is pure
/// Go on top of `net`.
use crate::value::{Value, ValueTag};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// A TCP listener or stream, wrapped behind `ValueTag::Socket`.
///
/// Each variant holds an `Option` so an explicit `(net/close! x)` can take
/// and drop the real OS resource immediately (deterministic, Go-`defer`-
/// style cleanup) without freeing the `Value`/`Box` wrapper itself --
/// refcount-based cleanup in `deallocate_value` still runs later, when the
/// last reference disappears, but by then finds `None` and safely does
/// nothing at the OS level. Without this split, an explicit close followed
/// by the eventual refcount-triggered cleanup would double-free the same
/// boxed value.
pub enum SocketHandle {
    Listener(Option<TcpListener>),
    Stream(Option<TcpStream>),
}

unsafe fn socket_handle(val: *mut Value) -> Option<*mut SocketHandle> {
    if val.is_null() || (*val).header().tag() != ValueTag::Socket {
        return None;
    }
    Some((*val).as_ptr() as *mut SocketHandle)
}

/// Closes the underlying OS resource in place, if not already closed.
/// Does not free the `Box<SocketHandle>` itself -- see `SocketHandle`'s
/// doc comment for why.
unsafe fn close_in_place(handle: *mut SocketHandle) {
    match &mut *handle {
        SocketHandle::Listener(slot) => {
            drop(slot.take());
        }
        SocketHandle::Stream(slot) => {
            drop(slot.take());
        }
    }
}

/// Called from `deallocate_value` once the last reference to a Socket
/// Value is released. Closes the OS resource if an explicit `(net/close!)`
/// hasn't already done so, then frees the `Box<SocketHandle>`.
pub unsafe fn release_socket_handle(handle: *mut SocketHandle) {
    if handle.is_null() {
        return;
    }
    close_in_place(handle);
    drop(Box::from_raw(handle));
}

fn wrap_handle(handle: SocketHandle) -> *mut Value {
    let raw = Box::into_raw(Box::new(handle));
    Value::from_ptr(ValueTag::Socket, raw as *mut u8)
}

/// Bind a TCP listener on `port` (all interfaces). `port_val` is a boxed
/// Clorus Long, matching the calling convention every other Value*-taking
/// builtin uses (Clorus source never passes raw, unboxed primitives).
/// Returns nil on failure (matches this runtime's existing convention for
/// simple I/O failures, e.g. clorus_read_line) -- callers that need the OS
/// error message should use a higher-level stdlib wrapper that checks for
/// nil and raises a proper Clorus exception with the details.
#[no_mangle]
pub extern "C" fn clorus_tcp_listen(port_val: *mut Value) -> *mut Value {
    unsafe {
        if port_val.is_null() || (*port_val).header().tag() != ValueTag::Long {
            return Value::nil();
        }
        let port = (*port_val).as_long();
        if !(0..=65535).contains(&port) {
            return Value::nil();
        }
        match TcpListener::bind(("0.0.0.0", port as u16)) {
            Ok(listener) => wrap_handle(SocketHandle::Listener(Some(listener))),
            Err(_) => Value::nil(),
        }
    }
}

/// Connect to `host_val:port_val` (both boxed Clorus values -- a String
/// and a Long respectively), returning a Socket Value wrapping the
/// connected stream. Returns nil on failure or bad argument types.
#[no_mangle]
pub extern "C" fn clorus_tcp_connect(host_val: *mut Value, port_val: *mut Value) -> *mut Value {
    unsafe {
        if host_val.is_null() || (*host_val).header().tag() != ValueTag::String {
            return Value::nil();
        }
        if port_val.is_null() || (*port_val).header().tag() != ValueTag::Long {
            return Value::nil();
        }
        let host = (*host_val).as_string().to_string();
        let port = (*port_val).as_long();
        if !(0..=65535).contains(&port) {
            return Value::nil();
        }
        match TcpStream::connect((host.as_str(), port as u16)) {
            Ok(stream) => wrap_handle(SocketHandle::Stream(Some(stream))),
            Err(_) => Value::nil(),
        }
    }
}

/// Block until a connection arrives on `listener`, returning a Socket
/// Value wrapping the accepted stream. Returns nil if `listener` isn't a
/// live listener handle (wrong tag, or already closed) or on accept error.
#[no_mangle]
pub extern "C" fn clorus_tcp_accept(listener_val: *mut Value) -> *mut Value {
    unsafe {
        let Some(handle) = socket_handle(listener_val) else {
            return Value::nil();
        };
        let SocketHandle::Listener(Some(listener)) = &*handle else {
            return Value::nil();
        };
        match listener.accept() {
            Ok((stream, _addr)) => wrap_handle(SocketHandle::Stream(Some(stream))),
            Err(_) => Value::nil(),
        }
    }
}

/// Read up to `max_bytes_val` (a boxed Clorus Long) from `stream`,
/// returning a Clorus string. Returns an empty string on EOF, nil on error
/// or if `stream` isn't a live stream handle. Not necessarily valid UTF-8
/// input is lossily converted -- HTTP headers/bodies this is aimed at are
/// ASCII/UTF-8 in practice; binary-safe reads are a follow-up if/when
/// something needs them.
#[no_mangle]
pub extern "C" fn clorus_tcp_read(stream_val: *mut Value, max_bytes_val: *mut Value) -> *mut Value {
    unsafe {
        if max_bytes_val.is_null() || (*max_bytes_val).header().tag() != ValueTag::Long {
            return Value::nil();
        }
        let max_bytes = (*max_bytes_val).as_long();
        if max_bytes <= 0 {
            return crate::string::clorus_string(std::ptr::null());
        }
        let Some(handle) = socket_handle(stream_val) else {
            return Value::nil();
        };
        let SocketHandle::Stream(Some(stream)) = &mut *handle else {
            return Value::nil();
        };
        let mut buf = vec![0u8; max_bytes as usize];
        match stream.read(&mut buf) {
            Ok(n) => {
                let s = String::from_utf8_lossy(&buf[..n]).into_owned();
                let c_str = std::ffi::CString::new(s).unwrap_or_default();
                crate::string::clorus_string(c_str.as_ptr())
            }
            Err(_) => Value::nil(),
        }
    }
}

/// Write a Clorus string's bytes to `stream`. Returns the number of bytes
/// written (as a Clorus long), or nil on error or if `stream` isn't a live
/// stream handle.
#[no_mangle]
pub extern "C" fn clorus_tcp_write(stream_val: *mut Value, data: *mut Value) -> *mut Value {
    unsafe {
        let Some(handle) = socket_handle(stream_val) else {
            return Value::nil();
        };
        let SocketHandle::Stream(Some(stream)) = &mut *handle else {
            return Value::nil();
        };
        if data.is_null() || (*data).header().tag() != ValueTag::String {
            return Value::nil();
        }
        let bytes = crate::string::value_to_rust_string(data);
        match stream.write(bytes.as_bytes()) {
            Ok(n) => Value::long(n as i64),
            Err(_) => Value::nil(),
        }
    }
}

/// Explicitly close a listener or stream immediately (Go's `defer
/// conn.Close()`, Clojure's `with-open`). Idempotent -- closing an
/// already-closed handle, or one whose last reference will later trigger
/// refcount-based cleanup anyway, is safe. Returns nil either way (no
/// meaningful failure mode to report -- an already-closed handle is not an
/// error).
#[no_mangle]
pub extern "C" fn clorus_tcp_close(handle_val: *mut Value) -> *mut Value {
    unsafe {
        if let Some(handle) = socket_handle(handle_val) {
            close_in_place(handle);
        }
    }
    Value::nil()
}
