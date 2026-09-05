//! Client side of the `lovely-injector-nx` C ABI
//! Use to get the game's Lua state and C API from the loader plugin once lovely is up

#![no_std]

use core::ffi::{c_char, c_int, c_void};

/// SHould be Bumped on incompatible changes to [`LuaApi`].
pub const API_VERSION: u32 = 1;

pub type LuaState = c_void;
pub type LuaCFunction = unsafe extern "C" fn(*mut LuaState) -> c_int;
pub type ReadyFn = unsafe extern "C" fn(state: *mut LuaState, api: *const LuaApi);

/// Lua 5.1 C API of the game's LuaJIT
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LuaApi {
    pub version: u32,
    pub size: u32,

    pub loadbuffer: unsafe extern "C" fn(*mut LuaState, *const u8, usize, *const c_char) -> c_int,
    pub call: unsafe extern "C" fn(*mut LuaState, c_int, c_int),
    pub pcall: unsafe extern "C" fn(*mut LuaState, c_int, c_int, c_int) -> c_int,
    pub error: unsafe extern "C" fn(*mut LuaState) -> c_int,
    pub gettop: unsafe extern "C" fn(*mut LuaState) -> c_int,
    pub settop: unsafe extern "C" fn(*mut LuaState, c_int),
    pub getfield: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char),
    pub setfield: unsafe extern "C" fn(*mut LuaState, c_int, *const c_char),
    pub settable: unsafe extern "C" fn(*mut LuaState, c_int),
    pub createtable: unsafe extern "C" fn(*mut LuaState, c_int, c_int),
    pub type_: unsafe extern "C" fn(*mut LuaState, c_int) -> c_int,
    pub tolstring: unsafe extern "C" fn(*mut LuaState, c_int, *mut usize) -> *const c_char,
    pub checklstring: unsafe extern "C" fn(*mut LuaState, c_int, *mut usize) -> *const c_char,
    pub pushvalue: unsafe extern "C" fn(*mut LuaState, c_int),
    pub pushnil: unsafe extern "C" fn(*mut LuaState),
    pub pushnumber: unsafe extern "C" fn(*mut LuaState, f64),
    pub pushboolean: unsafe extern "C" fn(*mut LuaState, c_int),
    pub pushstring: unsafe extern "C" fn(*mut LuaState, *const c_char),
    pub pushlstring: unsafe extern "C" fn(*mut LuaState, *const u8, usize),
    pub pushcclosure: unsafe extern "C" fn(*mut LuaState, LuaCFunction, c_int),
    pub register: unsafe extern "C" fn(*mut LuaState, *const c_char, *const c_void),
}

pub const SYMBOL_API_VERSION: &str = "lovely_nx_api_version";
pub const SYMBOL_ON_READY: &str = "lovely_nx_on_ready";

type ApiVersionFn = unsafe extern "C" fn() -> u32;
type OnReadyFn = unsafe extern "C" fn(ReadyFn) -> bool;

unsafe fn lookup(name: &str) -> Option<usize> {
    let mut buf = [0u8; 64];
    if name.len() >= buf.len() {
        return None;
    }

    buf[..name.len()].copy_from_slice(name.as_bytes());
    let mut addr: usize = 0;
    let rc = skyline::nn::ro::LookupSymbol(&mut addr, buf.as_ptr());
    if rc == 0 && addr != 0 {
        Some(addr)
    } else {
        None
    }
}

/// [`API_VERSION`] of the loaded loader plugin
pub fn api_version() -> Option<u32> {
    unsafe {
        let f: ApiVersionFn = core::mem::transmute(lookup(SYMBOL_API_VERSION)?);
        Some(f())
    }
}

/// Check if lovely is ready
pub fn on_ready(cb: ReadyFn) -> Result<(), &'static str> {
    match api_version() {
        None => return Err("lovely-injector-nx plugin is not loaded"),
        Some(v) if v != API_VERSION => return Err("lovely-injector-nx API version mismatch"),
        Some(_) => {}
    }
    unsafe {
        let f: OnReadyFn =
            core::mem::transmute(lookup(SYMBOL_ON_READY).ok_or("lovely_nx_on_ready not exported")?);
        if f(cb) {
            Ok(())
        } else {
            Err("lovely-injector-nx refused the callback ?")
        }
    }
}
