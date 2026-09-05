//! lovely-injector on Nintendo Switch
//!
//! Boot sequence:
//! 1. `main()` (called by skyline) scan the game build
//!    ([`offsets`]) and installs an inline hook on LuaJIT's `luaL_loadbuffer` ([`hook`]).
//! 2. The first time the game loads a Lua chunk, the hook mounts the SD card, initialises
//!    lovely with the mod directory `sd:/Balatro/Mods` and the Lua C API resolved from
//!    the offset table ([`lua_api`]), installs the Steamodded compatibility wedges
//!    ([`nxfs`]) and tells the other plugins the state is ready ([`plugins`]).
//! 3. Every subsequent chunk goes through `Lovely::apply_buffer_patches`, like on regular lovely.

#![allow(clippy::missing_safety_doc)]

mod hook;
mod lua_api;
mod nxfs;
mod offsets;
mod plugins;

/// Mirrors `%APPDATA%\Balatro\Mods` on PC.
pub const MOD_DIR: &str = "sd:/Balatro/Mods";

#[skyline::main(name = "lovely_injector_nx")]
pub fn main() {
    println!("[lovely-nx] plugin loaded (v{})", env!("CARGO_PKG_VERSION"));

    let Some(offs) = offsets::detect() else {
        println!("[lovely-nx] unsupported Balatro build: no offset table matched !");
        return;
    };

    println!("[lovely-nx] detected Balatro build '{}'", offs.name);

    hook::install();
}
