//! The game's Lua C API from the offset table.
//!
//! `lua_pcall`, `lua_error` and `lua_pushlstring` do not exist in the binary,
//! so re implemented here from LuaJIT 2.1's lj_api.c on top of the internals that do exist
//! Only these three touch LuaJIT's struct layout below

use core::ffi::{c_char, c_int, c_void};
use std::mem::transmute;
use std::sync::OnceLock;

use lovely_core::sys::{LuaLib, LuaState};
use lovely_nx_api::LuaApi;

use crate::offsets::{self, Offsets};

pub type LoadBuffer = unsafe extern "C" fn(*mut LuaState, *const u8, usize, *const u8) -> u32;
type LjVmPcall = unsafe extern "C" fn(*mut LuaState, *mut u64, c_int, isize) -> c_int;
type LjErrRun = unsafe extern "C" fn(*mut LuaState) -> !;
type LjStrNew = unsafe extern "C" fn(*mut LuaState, *const u8, usize) -> *mut c_void;
type LjGcStep = unsafe extern "C" fn(*mut LuaState) -> c_int;
type LjStateGrowstack = unsafe extern "C" fn(*mut LuaState, c_int);
type LuaPushNil = unsafe extern "C" fn(*mut LuaState);

/// Trampoline to the game's original functions
static ORIG_LOAD_BUFFER: OnceLock<LoadBuffer> = OnceLock::new();
static ORIG_VM_PCALL: OnceLock<LjVmPcall> = OnceLock::new();
static ORIG_ERR_RUN: OnceLock<LjErrRun> = OnceLock::new();

// LuaJIT 2.1 layout
const L_GLREF: usize = 0x10; // global_State *
const L_BASE: usize = 0x20; // TValue *base
const L_TOP: usize = 0x28; // TValue *top
const L_MAXSTACK: usize = 0x30; // TValue *maxstack
const L_STACK: usize = 0x38; // TValue *stack
const G_GC_TOTAL: usize = 0x10; // global_State.gc.total
const G_GC_THRESHOLD: usize = 0x18;
const G_HOOKMASK: usize = 0x91; // uint8_t hookmask
const HOOK_EVENTMASK: u8 = 0x0f;
const LJ_TNIL: u64 = 0xffff_ffff_ffff_ffff;
/// LuaJIT string tag
const LJ_TSTR_TAG: u64 = 0xfffd_8000_0000_0000;

#[inline(always)]
unsafe fn field<T: Copy>(l: *mut LuaState, off: usize) -> T {
    *((l as *mut u8).add(off) as *const T)
}

#[inline(always)]
unsafe fn set_field<T: Copy>(l: *mut LuaState, off: usize, v: T) {
    *((l as *mut u8).add(off) as *mut T) = v;
}

pub fn set_loadbuffer(orig: LoadBuffer) {
    let _ = ORIG_LOAD_BUFFER.set(orig);
}

/// The game original `luaL_loadbuffer`
pub fn original_lual_loadbuffer() -> LoadBuffer {
    *ORIG_LOAD_BUFFER
        .get()
        .expect("luaL_loadbuffer trampoline not captured yet")
}

/// lj_api.c `api_call_base`
unsafe fn api_call_base(l: *mut LuaState, nargs: c_int) -> *mut u64 {
    let top: *mut u64 = field(l, L_TOP);
    let base = top.sub(nargs as usize);
    set_field(l, L_TOP, top.add(1));
    let mut o = top;
    while o > base {
        *o = *o.sub(1);
        o = o.sub(1);
    }
    *o = LJ_TNIL;
    o.add(1)
}

/// lj_api.c `lua_pcall`
pub unsafe extern "C" fn lua_pcall(
    l: *mut LuaState,
    nargs: c_int,
    nresults: c_int,
    errfunc: c_int,
) -> c_int {
    let g: *mut u8 = field(l, L_GLREF);
    let oldh = *g.add(G_HOOKMASK) & !HOOK_EVENTMASK;
    let ef: isize = if errfunc == 0 {
        0
    } else {
        let o: *mut u64 = if errfunc > 0 {
            field::<*mut u64>(l, L_BASE).add(errfunc as usize - 1)
        } else {
            field::<*mut u64>(l, L_TOP).offset(errfunc as isize)
        };
        (o as isize) - (field::<*mut u64>(l, L_STACK) as isize)
    };
    let base = api_call_base(l, nargs);
    let status = (ORIG_VM_PCALL.get().unwrap())(l, base, nresults + 1, ef);
    if status != 0 {
        let h = g.add(G_HOOKMASK);
        *h = (*h & HOOK_EVENTMASK) | oldh;
    }
    status
}

/// lj_api.c `lua_error`
pub unsafe extern "C" fn lua_error(l: *mut LuaState) -> c_int {
    (ORIG_ERR_RUN.get().unwrap())(l)
}

/// lj_api.c `lua_pushlstring`
pub unsafe fn push_lstring(l: *mut LuaState, s: &[u8]) {
    let o = offsets::get();
    let lj_str_new: LjStrNew = transmute(offsets::abs(o.lj_str_new));
    let lj_gc_step: LjGcStep = transmute(offsets::abs(o.lj_gc_step));
    let lj_state_growstack: LjStateGrowstack = transmute(offsets::abs(o.lj_state_growstack));

    let g: *mut u8 = field(l, L_GLREF);
    let total = *(g.add(G_GC_TOTAL) as *const u64);
    let threshold = *(g.add(G_GC_THRESHOLD) as *const u64);
    if total >= threshold {
        lj_gc_step(l);
    }
    let str = lj_str_new(l, s.as_ptr(), s.len());
    let top: *mut u64 = field(l, L_TOP);
    *top = (str as u64) | LJ_TSTR_TAG;
    let top = top.add(1);
    set_field(l, L_TOP, top);
    if top >= field::<*mut u64>(l, L_MAXSTACK) {
        lj_state_growstack(l, 1);
    }
}

unsafe extern "C" fn pushlstring_c(l: *mut c_void, s: *const u8, len: usize) {
    push_lstring(l as *mut LuaState, core::slice::from_raw_parts(s, len));
}

unsafe extern "C" fn loadbuffer_c(
    l: *mut c_void,
    buf: *const u8,
    size: usize,
    name: *const c_char,
) -> c_int {
    original_lual_loadbuffer()(l as *mut LuaState, buf, size, name as *const u8) as c_int
}

pub unsafe fn push_nil(l: *mut LuaState) {
    let f: LuaPushNil = transmute(offsets::abs(offsets::get().lua_pushnil));
    f(l);
}

/// lovely core's API table
pub unsafe fn lualib(o: &Offsets) -> LuaLib {
    macro_rules! f {
        ($field:ident) => {
            transmute(offsets::abs(o.$field))
        };
    }
    let _ = ORIG_VM_PCALL.set(f!(lj_vm_pcall));
    let _ = ORIG_ERR_RUN.set(f!(lj_err_run));
    LuaLib {
        lua_call: f!(lua_call),
        lua_pcall,
        lua_getfield: f!(lua_getfield),
        lua_setfield: f!(lua_setfield),
        lua_gettop: f!(lua_gettop),
        lua_settop: f!(lua_settop),
        lua_pushvalue: f!(lua_pushvalue),
        lua_pushcclosure: f!(lua_pushcclosure),
        lua_tolstring: f!(lua_tolstring),
        lua_type: f!(lua_type),
        lua_pushstring: f!(lua_pushstring),
        lua_pushnumber: f!(lua_pushnumber),
        lua_pushboolean: f!(lua_pushboolean),
        lua_settable: f!(lua_settable),
        lua_createtable: f!(lua_createtable),
        lua_error,
        lual_register: f!(lual_register),
        lual_checklstring: f!(lual_checklstring),
    }
}

static LUA_API: OnceLock<LuaApi> = OnceLock::new();

/// The table handed to other plugins
pub fn exported(o: &Offsets) -> &'static LuaApi {
    LUA_API.get_or_init(|| unsafe {
        macro_rules! f {
            ($field:ident) => {
                transmute(offsets::abs(o.$field))
            };
        }
        LuaApi {
            version: lovely_nx_api::API_VERSION,
            size: core::mem::size_of::<LuaApi>() as u32,
            loadbuffer: loadbuffer_c,
            call: f!(lua_call),
            pcall: transmute(lua_pcall as *const ()),
            error: transmute(lua_error as *const ()),
            gettop: f!(lua_gettop),
            settop: f!(lua_settop),
            getfield: f!(lua_getfield),
            setfield: f!(lua_setfield),
            settable: f!(lua_settable),
            createtable: f!(lua_createtable),
            type_: f!(lua_type),
            tolstring: f!(lua_tolstring),
            checklstring: f!(lual_checklstring),
            pushvalue: f!(lua_pushvalue),
            pushnil: f!(lua_pushnil),
            pushnumber: f!(lua_pushnumber),
            pushboolean: f!(lua_pushboolean),
            pushstring: f!(lua_pushstring),
            pushlstring: pushlstring_c,
            pushcclosure: f!(lua_pushcclosure),
            register: f!(lual_register),
        }
    })
}
