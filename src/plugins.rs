//! Exported C ABI for other skyline plugins

use core::ffi::c_void;
use std::sync::Mutex;

use lovely_nx_api::{LuaApi, ReadyFn, API_VERSION};

const MAX_CALLBACKS: usize = 16;

struct Registry {
    callbacks: Vec<ReadyFn>,
    ready: Option<(usize, &'static LuaApi)>,
}

static REGISTRY: Mutex<Registry> = Mutex::new(Registry {
    callbacks: Vec::new(),
    ready: None,
});

#[no_mangle]
pub extern "C" fn lovely_nx_api_version() -> u32 {
    API_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn lovely_nx_on_ready(cb: ReadyFn) -> bool {
    let ready = {
        let mut r = REGISTRY.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(ready) = r.ready {
            Some(ready)
        } else {
            if r.callbacks.len() >= MAX_CALLBACKS {
                return false;
            }
            r.callbacks.push(cb);
            None
        }
    };

    if let Some((state, api)) = ready {
        cb(state as *mut c_void, api);
    }
    true
}

pub unsafe fn fire_callbacks(state: *mut c_void, api: &'static LuaApi) {
    let callbacks = {
        let mut r = REGISTRY.lock().unwrap_or_else(|p| p.into_inner());
        r.ready = Some((state as usize, api));
        std::mem::take(&mut r.callbacks)
    };
    println!(
        "[lovely-nx] main Lua state ready, notifying {} plugin(s)",
        callbacks.len()
    );
    for cb in callbacks {
        cb(state, api);
    }
}
