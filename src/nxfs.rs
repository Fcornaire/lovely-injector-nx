//! Steamodded compatibility, a global `nxfs` table over `std::fs`

use core::ffi::{c_char, c_int, c_void};
use std::io::Write;
use std::time::UNIX_EPOCH;

use lovely_core::sys::{self as lua, LuaFunc, LuaState, LUA_GLOBALSINDEX};

use crate::lua_api::{original_lual_loadbuffer, push_lstring, push_nil};

const LUA_TSTRING: c_int = 4;

const NATIVE_FS_WEDGE: &str = include_str!("nativefs_nx.lua");
const SOCKET_WEDGE: &str = include_str!("socket_nx.lua");
const LUA_TNIL: c_int = 0;

unsafe fn arg_bytes<'a>(l: *mut LuaState, idx: c_int) -> &'a [u8] {
    let mut len = 0usize;
    let p = lua::lual_checklstring(l, idx, &mut len);
    core::slice::from_raw_parts(p as *const u8, len)
}

unsafe fn arg_path(l: *mut LuaState, idx: c_int) -> String {
    String::from_utf8_lossy(arg_bytes(l, idx)).into_owned()
}

unsafe fn fail(l: *mut LuaState, msg: String) -> c_int {
    push_nil(l);
    push_lstring(l, msg.as_bytes());
    2
}

fn mtime(m: &std::fs::Metadata) -> f64 {
    m.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as f64)
        .unwrap_or(0.0)
}

/// `nxfs.read(path) -> data | nil, err`
unsafe extern "C" fn nx_read(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    match std::fs::read(&p) {
        Ok(d) => {
            push_lstring(l, &d);
            1
        }
        Err(e) => fail(l, format!("{p}: {e}")),
    }
}

/// `nxfs.write(path, data [, "a"]) -> true | nil, err`
unsafe extern "C" fn nx_write(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    let data = arg_bytes(l, 2);
    let append = lua::lua_type(l, 3) == LUA_TSTRING && arg_bytes(l, 3) == b"a";
    let r = if append {
        std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&p)
            .and_then(|mut f| f.write_all(data))
    } else {
        std::fs::write(&p, data)
    };
    match r {
        Ok(()) => {
            lua::lua_pushboolean(l, 1);
            1
        }
        Err(e) => fail(l, format!("{p}: {e}")),
    }
}

/// `nxfs.list(dir) -> { names... } | nil, err`
unsafe extern "C" fn nx_list(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    let rd = match std::fs::read_dir(&p) {
        Ok(rd) => rd,
        Err(e) => return fail(l, format!("{p}: {e}")),
    };
    let mut names: Vec<Vec<u8>> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned().into_bytes())
        .collect();
    names.sort();
    lua::lua_createtable(l, names.len() as c_int, 0);
    for (i, n) in names.iter().enumerate() {
        lua::lua_pushnumber(l, (i + 1) as f64);
        push_lstring(l, n);
        lua::lua_settable(l, -3);
    }
    1
}

/// `nxfs.info(path) -> "file"|"directory", size, modtime | nil`
unsafe extern "C" fn nx_info(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    let p = p.trim_end_matches('/');
    let p = if p.ends_with(':') {
        format!("{p}/")
    } else {
        p.to_string()
    };
    match std::fs::metadata(&p) {
        Ok(m) => {
            let (ty, size): (&[u8], f64) = if m.is_dir() {
                (b"directory", 0.0)
            } else {
                (b"file", m.len() as f64)
            };
            push_lstring(l, ty);
            lua::lua_pushnumber(l, size);
            lua::lua_pushnumber(l, mtime(&m));
            3
        }
        Err(_) => {
            push_nil(l);
            1
        }
    }
}

/// `nxfs.mkdir(path) -> true | nil, err`
unsafe extern "C" fn nx_mkdir(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    match std::fs::create_dir(p.trim_end_matches('/')) {
        Ok(()) => {
            lua::lua_pushboolean(l, 1);
            1
        }
        Err(e) => fail(l, format!("{p}: {e}")),
    }
}

/// `nxfs.remove(path) -> true | nil, err`
unsafe extern "C" fn nx_remove(l: *mut LuaState) -> c_int {
    let p = arg_path(l, 1);
    let r = match std::fs::metadata(&p) {
        Ok(m) if m.is_dir() => std::fs::remove_dir(&p),
        Ok(_) => std::fs::remove_file(&p),
        Err(e) => Err(e),
    };
    match r {
        Ok(()) => {
            lua::lua_pushboolean(l, 1);
            1
        }
        Err(e) => fail(l, format!("{p}: {e}")),
    }
}

/// `nxfs.log(msg)`: print to the skyline logger
unsafe extern "C" fn nx_log(l: *mut LuaState) -> c_int {
    let msg = String::from_utf8_lossy(arg_bytes(l, 1)).into_owned();
    println!("[lovely-nx] lua: {msg}");
    0
}

#[repr(C)]
struct LuaLReg {
    name: *const c_char,
    func: Option<LuaFunc>,
}
unsafe impl Sync for LuaLReg {}

macro_rules! reg {
    ($name:literal, $f:ident) => {
        LuaLReg {
            name: concat!($name, "\0").as_ptr() as *const c_char,
            func: Some($f),
        }
    };
}

static REGS: [LuaLReg; 8] = [
    reg!("read", nx_read),
    reg!("write", nx_write),
    reg!("list", nx_list),
    reg!("info", nx_info),
    reg!("mkdir", nx_mkdir),
    reg!("remove", nx_remove),
    reg!("log", nx_log),
    LuaLReg {
        name: core::ptr::null(),
        func: None,
    },
];

/// Registers `nxfs` and preloads the `nativefs` and `socket` wedges.
pub unsafe fn install(l: *mut LuaState) {
    let top = lua::lua_gettop(l);

    lua::lual_register(
        l,
        b"nxfs\0".as_ptr() as *const char,
        REGS.as_ptr() as *const c_void,
    );
    lua::lua_settop(l, top);

    let rc = original_lual_loadbuffer()(
        l,
        NATIVE_FS_WEDGE.as_ptr(),
        NATIVE_FS_WEDGE.len(),
        b"=[lovely-nx nativefs \"nativefs_nx.lua\"]\0".as_ptr(),
    );

    if rc != 0 {
        let mut len = 0usize;
        let p = lua::lua_tolstring(l, -1, &mut len);
        let err = String::from_utf8_lossy(core::slice::from_raw_parts(p as *const u8, len));
        println!("[lovely-nx] nativefs wedge failed to compile: {err}");
        lua::lua_settop(l, top);
        return;
    }

    lua::lua_getfield(l, LUA_GLOBALSINDEX, b"package\0".as_ptr() as *const c_char);
    lua::lua_getfield(l, -1, b"preload\0".as_ptr() as *const c_char);

    for name in [b"nativefs\0".as_slice(), b"SMODS.nativefs\0".as_slice()] {
        lua::lua_pushvalue(l, -3);
        lua::lua_setfield(l, -2, name.as_ptr() as *const c_char);
    }
    lua::lua_settop(l, top);
    println!("[lovely-nx] nativefs replaced by nxfs (package.preload.nativefs / SMODS.nativefs)");

    preload_if_missing(
        l,
        "socket",
        SOCKET_WEDGE,
        b"=[lovely-nx socket \"socket_nx.lua\"]\0",
    );
}

unsafe fn preload_if_missing(l: *mut LuaState, name: &str, src: &str, chunkname: &[u8]) {
    let top = lua::lua_gettop(l);
    let cname = format!("{name}\0");
    lua::lua_getfield(l, LUA_GLOBALSINDEX, b"package\0".as_ptr() as *const c_char);
    lua::lua_getfield(l, -1, b"preload\0".as_ptr() as *const c_char);
    lua::lua_getfield(l, -1, cname.as_ptr() as *const c_char);
    let missing = lua::lua_type(l, -1) == LUA_TNIL;
    lua::lua_settop(l, top + 2);
    if missing {
        let rc = original_lual_loadbuffer()(l, src.as_ptr(), src.len(), chunkname.as_ptr());
        if rc != 0 {
            let mut len = 0usize;
            let p = lua::lua_tolstring(l, -1, &mut len);
            let err = String::from_utf8_lossy(core::slice::from_raw_parts(p as *const u8, len));
            println!("[lovely-nx] {name} wedge failed to compile: {err}");
        } else {
            lua::lua_setfield(l, -2, cname.as_ptr() as *const c_char);
            println!("[lovely-nx] package.preload.{name} provided by the plugin");
        }
    }
    lua::lua_settop(l, top);
}
