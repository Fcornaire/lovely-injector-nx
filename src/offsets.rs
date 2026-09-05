//! `.text` offsets of the Lua C API in Balatro's `main` (Balatro o switch use LuaJIT)

use skyline::hooks::{getRegionAddress, Region};
use std::{ptr::read_volatile, sync::OnceLock};

static TABLES: [&Offsets; 1] = [&V1_1_3];
static DETECTED: OnceLock<Option<&'static Offsets>> = OnceLock::new();

/// One function, offset from `.text` + expected first instruction word
#[derive(Clone, Copy, Debug)]
pub struct Fn {
    pub off: usize,
    pub word: u32,
}

#[derive(Debug)]
pub struct Offsets {
    pub name: &'static str,

    pub lual_loadbuffer: Fn,
    pub lual_newstate: Fn,
    pub lua_call: Fn,
    pub lj_vm_pcall: Fn,
    pub lua_getfield: Fn,
    pub lua_setfield: Fn,
    pub lua_gettop: Fn,
    pub lua_settop: Fn,
    pub lua_pushvalue: Fn,
    pub lua_pushcclosure: Fn,
    pub lua_tolstring: Fn,
    pub lua_type: Fn,
    pub lua_pushstring: Fn,
    pub lua_pushnumber: Fn,
    pub lua_pushboolean: Fn,
    pub lua_settable: Fn,
    pub lua_createtable: Fn,
    pub lj_err_run: Fn,
    pub lual_register: Fn,
    pub lual_checklstring: Fn,
    pub lua_pushnil: Fn,
    pub lj_str_new: Fn,
    pub lj_gc_step: Fn,
    pub lj_state_growstack: Fn,
}

impl Offsets {
    fn all(&self) -> [(&'static str, Fn); 24] {
        [
            ("luaL_loadbuffer", self.lual_loadbuffer),
            ("luaL_newstate", self.lual_newstate),
            ("lua_call", self.lua_call),
            ("lj_vm_pcall", self.lj_vm_pcall),
            ("lua_getfield", self.lua_getfield),
            ("lua_setfield", self.lua_setfield),
            ("lua_gettop", self.lua_gettop),
            ("lua_settop", self.lua_settop),
            ("lua_pushvalue", self.lua_pushvalue),
            ("lua_pushcclosure", self.lua_pushcclosure),
            ("lua_tolstring", self.lua_tolstring),
            ("lua_type", self.lua_type),
            ("lua_pushstring", self.lua_pushstring),
            ("lua_pushnumber", self.lua_pushnumber),
            ("lua_pushboolean", self.lua_pushboolean),
            ("lua_settable", self.lua_settable),
            ("lua_createtable", self.lua_createtable),
            ("lj_err_run", self.lj_err_run),
            ("luaL_register", self.lual_register),
            ("luaL_checklstring", self.lual_checklstring),
            ("lua_pushnil", self.lua_pushnil),
            ("lj_str_new", self.lj_str_new),
            ("lj_gc_step", self.lj_gc_step),
            ("lj_state_growstack", self.lj_state_growstack),
        ]
    }

    /// True when every function's first instruction matches the live `.text`.
    fn matches(&self, text: usize) -> bool {
        for (name, f) in self.all() {
            if f.off == 0 {
                println!("[lovely-nx] table '{}': {} has no offset", self.name, name);
                return false;
            }

            let word = unsafe { read_volatile((text + f.off) as *const u32) };
            if word != f.word {
                println!(
                    "[lovely-nx] table '{}': {} @+{:#x} word {:#010x} != expected {:#010x}",
                    self.name, name, f.off, word, f.word
                );
                return false;
            }
        }
        true
    }
}

/// Balatro Switch 1.1.3
pub static V1_1_3: Offsets = Offsets {
    name: "v1.1.3",
    lual_loadbuffer: Fn {
        off: 0x146f0,
        word: 0xd10083ff,
    },
    lual_newstate: Fn {
        off: 0x16960,
        word: 0xd100c3ff,
    },
    lua_call: Fn {
        off: 0x131e0,
        word: 0xf9401408,
    },
    lj_vm_pcall: Fn {
        off: 0x55848,
        word: 0xd10343ff,
    },
    lua_getfield: Fn {
        off: 0x11370,
        word: 0xa9bd7bfd,
    },
    lua_setfield: Fn {
        off: 0x12660,
        word: 0xa9bd7bfd,
    },
    lua_gettop: Fn {
        off: 0xd190,
        word: 0xa9422009,
    },
    lua_settop: Fn {
        off: 0xd1a0,
        word: 0xa9be7bfd,
    },
    lua_pushvalue: Fn {
        off: 0xd730,
        word: 0xf9401408,
    },
    lua_pushcclosure: Fn {
        off: 0x10800,
        word: 0xa9bc7bfd,
    },
    lua_tolstring: Fn {
        off: 0xf7f0,
        word: 0xa9bd7bfd,
    },
    lua_type: Fn {
        off: 0xd810,
        word: 0x71000428,
    },
    lua_pushstring: Fn {
        off: 0x106d0,
        word: 0xa9be7bfd,
    },
    lua_pushnumber: Fn {
        off: 0x10620,
        word: 0xf9401408,
    },
    lua_pushboolean: Fn {
        off: 0x10a30,
        word: 0x7100003f,
    },
    lua_settable: Fn {
        off: 0x124d0,
        word: 0xa9be7bfd,
    },
    lua_createtable: Fn {
        off: 0x10c00,
        word: 0xa9bd7bfd,
    },
    lj_err_run: Fn {
        off: 0x17b0,
        word: 0xa9be7bfd,
    },
    lual_register: Fn {
        off: 0x15b60,
        word: 0xa9bc7bfd,
    },
    lual_checklstring: Fn {
        off: 0xf9b0,
        word: 0xa9bd7bfd,
    },
    lua_pushnil: Fn {
        off: 0x105f0,
        word: 0xf9401408,
    },
    lj_str_new: Fn {
        off: 0x2680,
        word: 0xa9ba7bfd,
    },
    lj_gc_step: Fn {
        off: 0x760,
        word: 0xa9bd7bfd,
    },
    lj_state_growstack: Fn {
        off: 0x8580,
        word: 0xa9ba7bfd,
    },
};

/// Base of the game `.text` at runtime
pub fn text_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

/// Absolute runtime address of an offset-table entry
pub fn abs(f: Fn) -> usize {
    text_base() + f.off
}

/// Pick the offset table matching the running build if any or cached
pub fn detect() -> Option<&'static Offsets> {
    *DETECTED.get_or_init(|| {
        let text = text_base();
        TABLES.iter().copied().find(|t| t.matches(text))
    })
}

/// The detected table, only valid after `detect()` returned `Some`
pub fn get() -> &'static Offsets {
    detect().expect("offsets::get() called without a detected build")
}
