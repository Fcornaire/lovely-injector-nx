# lovely-injector-nx

[![Support me on Patreon](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fshieldsio-patreon.vercel.app%2Fapi%3Fusername%3DDShadModdingAdventure%26type%3Dpatrons&style=for-the-badge)](https://patreon.com/DShadModdingAdventure)
[![Contributors][contributors-shield]][contributors-url]
[![Download][download-shield]][download-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![GPL3 License][license-shield]][license-url]

[lovely-injector](https://github.com/ethangreen-dev/lovely-injector) for Balatro on
Nintendo Switch, as a [skyline](https://github.com/skyline-dev/skyline) plugin

With it,lovely mods (Steamodded and anything built on it) load from the SD card exactly as they
do from `%APPDATA%\Balatro\Mods` on PC

Only the current game version (1.1.3) is supported

The repo can be summatized in 3 things:

- the plugin itself (`src/`): hooks the game, runs lovely, provides what Steamodded needs from the platform
- `mods/switch-perf/`: a lovely mod with the performance patches the Switch needs to run
  modded Balatro at a playable frame rate (see [Performance](#performance))
- `api/`: a C ABI for other skyline plugins that want native code attached to the game's
  Lua state, [balatro-vs](https://github.com/Fcornaire/balatro-vs) use it for example

## Install

Grab from the release the zip and extract it onto your SD card, the zip should already have the correct layout :

```
sd:/atmosphere/contents/0100CD801CE5E000/
  exefs/main.npdm
  exefs/subsdk9
  romfs/skyline/plugins/liblovely_injector_nx.nro
sd:/Balatro/Mods/
  switch-perf/lovely/perf.toml
  Steamodded/                                  a copy of Steamodded (1.0.0-beta tested)
  <other lovely mods>/                         same layout as on PC
```

`main.npdm` and `subsdk9` come from this repo's `exefs/` folder, not from the skyline
release (those crash Balatro at boot); `perf.toml` from `mods/switch-perf/`;
the `.nro` from the Releases page.

## How it works

| Platform | Injection                           | Lovely runtime                                 | Lua symbols                 |
| -------- | ----------------------------------- | ---------------------------------------------- | --------------------------- |
| Windows  | `winmm.dll` proxy                   | `lovely-win`                                   | `GetProcAddress(lua51.dll)` |
| Android  | `package.loadlib`                   | Lovely Mobile Maker                            | `dlsym(liblove.so)`         |
| Switch   | Skyline plugin loaded before `main` | Linked in (`lovely-core`, no default features) | offset table                |

Balatro on Switch is a custom LÖVE with LuaJIT 2.1.0-beta3
Offsets where found with Ghidra and turned into callable entries
We then hook `luaL_loadbuffer` and on the first chunk load , wich mount the SD card and start lovely

## Finding the Lua API offsets

`src/offsets.rs` has to be redone when a game update changes `main`. Everything was
done in Ghidra 10.2.2 with the [SwitchLoader](https://github.com/adubbz/ghidra-switch-loader) extension on the 1.1.3 `main`.

Nothing Lua related is exported, so the idea is basically find a good function candidate from the open source [LÖVE](https://github.com/love2d/love) framework in Ghidra and then read the assembly or the decompiled one to find the lua_x function needed. (Note: in Ghidra 10.2.2 the function pointer tables show no references, because the SwitchLoader build for it does not apply the binary's `DT_RELR` relocations and the pointers stay image-relative. Either search the string's image offset as a little-endian qword with Search > Memory, or fix the relocations once with a script that adds the image base to every `DT_RELR` target and types it as a pointer)

For example, lets say we want to find offset to call `luaL_checkstring` :

- Find a candidate in [LÖVE](https://github.com/love2d/love) , for example `w_getRealDirectory`
- Search for it in Ghidra
- Looking at the original [w_getRealDirectory](https://github.com/love2d/love/blob/a0ea2596798c3517d9117a98dc02871a11ed88cd/src/modules/filesystem/wrap_Filesystem.cpp#L506) , `luaL_checkstring` is used in the first instructions, so we can deduce the offset from ghidra for that function from there (`luaL_checkstring` is a macro, the function actually called is `luaL_checklstring(L, 1, NULL)`)

## Performance

The Switch build of Balatro ships LuaJIT with the JIT compiler compiled out, so every line of Lua a mod
adds runs interpreted, on a 1 GHz core. Vanilla Balatro run fine but with steamodded, it start to run slower.

Two things for a better experience :

### `mods/switch-perf`

A lovely mod (`Mods/switch-perf/lovely/perf.toml`) with six patches to gain some cpu usage

### CPU overclock (sys-clk)

Install [sys-clk](https://github.com/retronx-team/sys-clk)
(sysmodule to `atmosphere/contents/00FF0000636C6BFF/` with `flags/boot2.flag`, overlay to
`switch/.overlays/`) and add to `config/sys-clk/config.ini`:

```ini
[0100CD801CE5E000]
docked_cpu=1785
docked_mem=1600
handheld_charging_cpu=1785
handheld_charging_mem=1600
handheld_cpu=1785
handheld_mem=1600
```

sys-clk starts at boot, applies the profile whenever Balatro is running and restores stock
clocks when it exits. A versus match goes from about 25 to 37 fps, a solo run from 35 to 57

It may be possible to gain more an reach 60 but it would probably need to ditch the embeded LuaJit for something that run better

## Native plugins

A lovely mod is Lua. If a mod also needs native code (like [balatro-vs](https://github.com/Fcornaire/balatro-vs)), on PC and Android that code is a library the game or the mod loads (`winmm.dll`,
`package.loadlib`). On the Switch it is a second skyline plugin, and it gets the game's Lua
state from this one through `api/` (crate `lovely-nx-api`):

```rust
use core::ffi::c_void;
use lovely_nx_api::LuaApi;

#[skyline::main(name = "my_plugin")]
pub fn main() {
    if let Err(e) = lovely_nx_api::on_ready(ready) {
        println!("[my-plugin] {e}");
    }
}

unsafe extern "C" fn ready(state: *mut c_void, api: *const LuaApi) {
    let api = &*api;
    // ...
}
```

`ready` runs once, on the game's main thread, at the first Lua chunk load: lovely is up,
every mod's module patches are injected, the standard library is open, the SD card is
mounted.

## Build

`cargo install cargo-skyline`, then:

```sh
cargo skyline build --release
```

[contributors-shield]: https://img.shields.io/github/contributors/Fcornaire/lovely-injector-nx.svg?style=for-the-badge
[contributors-url]: https://github.com/Fcornaire/lovely-injector-nx/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/Fcornaire/lovely-injector-nx.svg?style=for-the-badge
[forks-url]: https://github.com/Fcornaire/lovely-injector-nx/network/members
[stars-shield]: https://img.shields.io/github/stars/Fcornaire/lovely-injector-nx.svg?style=for-the-badge
[stars-url]: https://github.com/Fcornaire/lovely-injector-nx/stargazers
[issues-shield]: https://img.shields.io/github/issues/Fcornaire/lovely-injector-nx.svg?style=for-the-badge
[issues-url]: https://github.com/Fcornaire/lovely-injector-nx/issues
[license-shield]: https://img.shields.io/github/license/Fcornaire/lovely-injector-nx.svg?style=for-the-badge
[download-shield]: https://img.shields.io/github/downloads/Fcornaire/lovely-injector-nx/total?style=for-the-badge
[download-url]: https://github.com/Fcornaire/lovely-injector-nx/releases
[license-url]: https://github.com/Fcornaire/lovely-injector-nx/blob/master/LICENSE.txt
