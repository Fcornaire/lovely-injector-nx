use std::panic::set_hook;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use lovely_core::sys::LuaState;
use lovely_core::{Lovely, RUNTIME};
use skyline::nn::fs::MountSdCardForDebug;

use crate::lua_api::lualib;
use crate::{lua_api, nxfs, offsets, plugins};

static SD_CARD_DISABLED: AtomicBool = AtomicBool::new(false);

/// Start lovely on the first chunk load, then routes every chunk through `apply_buffer_patches`
#[skyline::hook(offset = offsets::get().lual_loadbuffer.off)]
unsafe extern "C" fn lual_loadbuffer_hook(
    state: *mut LuaState,
    buf: *const u8,
    size: usize,
    name: *const u8,
) -> u32 {
    if SD_CARD_DISABLED.load(Ordering::Relaxed) {
        return lua_api::original_lual_loadbuffer()(state, buf, size, name);
    }

    let is_lovely_running = RUNTIME.get().is_some();
    if !is_lovely_running {
        let orig: lua_api::LoadBuffer = original!();
        lua_api::set_loadbuffer(orig);

        let rc = MountSdCardForDebug(b"sd\0".as_ptr());
        if !Path::new("sd:/").is_dir() {
            println!("[lovely-nx] sd:/ is not accessible");
            SD_CARD_DISABLED.store(true, Ordering::Relaxed);
            return orig(state, buf, size, name);
        }

        set_hook(Box::new(|info| {
            println!("[lovely-nx] lovely panic: {info}");
        }));

        let lua = lualib(offsets::get());
        let dump_all = Path::new(crate::MOD_DIR).join("dump_all").exists();
        Lovely::init_with_mod_dir(
            &|a, b, c, d, _mode| lua_api::original_lual_loadbuffer()(a, b, c, d),
            lua,
            dump_all,
            Some(PathBuf::from(crate::MOD_DIR)),
        );
        println!("[lovely-nx] lovely initialised, mods at {}", crate::MOD_DIR);
    }

    let rc = RUNTIME
        .get()
        .unwrap()
        .apply_buffer_patches(state, buf, size, name, core::ptr::null());
    if !is_lovely_running && !SD_CARD_DISABLED.load(Ordering::Relaxed) {
        nxfs::install(state);
        plugins::fire_callbacks(state as *mut _, lua_api::exported(offsets::get()));
    }
    rc
}

pub fn install() {
    skyline::install_hook!(lual_loadbuffer_hook);
    println!("[lovely-nx] luaL_loadbuffer hooked");
}
