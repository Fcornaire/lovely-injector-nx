-- nativefs replacement for Balatro on Switch
-- Same public API as Steamodded's
-- Switch paths are "<mount>:/..." (`sd:/Balatro/Mods/...`)

local nxfs = nxfs
assert(nxfs, "nxfs native table missing")

local nativefs = {}

local SD_ROOT = "sd:/Balatro"
local cwd = SD_ROOT

local function log(msg) nxfs.log("[nativefs] " .. msg) end

local function splitMount(p)
    local mount, rest = p:match("^(%a[%w_]*:)/*(.*)$")
    return mount, rest
end

local function normalize(p)
    p = p:gsub("\\", "/")
    local mount, rest = splitMount(p)
    if not mount then mount, rest = "", p end
    local parts = {}
    for seg in rest:gmatch("[^/]+") do
        if seg == ".." then
            if #parts > 0 then table.remove(parts) end
        elseif seg ~= "." then
            parts[#parts + 1] = seg
        end
    end
    return mount .. "/" .. table.concat(parts, "/")
end

-- Absolute nn::fs path for a nativefs path
local function resolve(p)
    if type(p) ~= "string" then
        error("bad path (string expected, got " .. type(p) .. ")", 3)
    end

    p = p:gsub("\\", "/")

    if splitMount(p) then return normalize(p) end
    if p:sub(1, 1) == "/" then return normalize(SD_ROOT .. p) end
    return normalize(cwd .. "/" .. p)
end

local function leaf(p)
    p = p:gsub("\\", "/")
    return p:match("([^/]*)$")
end

-----------------------------------------------------------------------------
-- File object

local File = {}
File.__index = File

function File:open(mode)
    if self._mode ~= 'c' then return false, "File " .. self._name .. " is already open" end
    local path = resolve(self._name)
    if mode == 'r' then
        local data, err = nxfs.read(path)
        if not data then return false, "Could not open " .. self._name .. " in mode r: " .. tostring(err) end
        self._buf = data
    elseif mode == 'w' then
        self._buf = ""
    elseif mode == 'a' then
        self._buf = nxfs.read(path) or ""
    else
        return false, "Invalid open mode for " .. self._name .. ": " .. tostring(mode)
    end
    self._mode, self._pos, self._dirty = mode, 0, false
    return true
end

function File:flush()
    if (self._mode == 'w' or self._mode == 'a') and self._dirty then
        local ok, err = nxfs.write(resolve(self._name), self._buf, "w")
        self._dirty = false
        if not ok then return false, err end
    end
    return true
end

function File:close()
    if self._mode == 'c' then return false, "File is not open" end
    local ok, err = self:flush()
    self._mode, self._buf = 'c', nil
    if not ok then return false, err end
    return true
end

function File:isOpen() return self._mode ~= 'c' end

function File:getMode() return self._mode end

function File:getFilename() return self._name end

function File:getBuffer() return self._bufferMode, self._bufferSize end

function File:setBuffer(mode, size)
    self._bufferMode, self._bufferSize = mode, size or 0; return true
end

function File:tell() return self._pos end

function File:isEOF() return not self._buf or self._pos >= #self._buf end

function File:seek(pos)
    if pos < 0 or (self._buf and pos > #self._buf) then return false end
    self._pos = pos
    return true
end

function File:getSize()
    if self._buf then return #self._buf end
    local _, size = nxfs.info(resolve(self._name))
    return size or 0
end

function File:read(containerOrBytes, bytes)
    if self._mode ~= 'r' then return nil, 0 end
    local container, n
    if bytes ~= nil then container, n = containerOrBytes, bytes else container, n = 'string', containerOrBytes end
    if container ~= 'string' and container ~= 'data' then error("Invalid container type: " .. tostring(container)) end
    local avail = #self._buf - self._pos
    if n == nil or n == 'all' then n = avail else n = math.max(0, math.min(avail, n)) end
    local str = self._buf:sub(self._pos + 1, self._pos + n)
    self._pos = self._pos + n
    if container == 'data' then return love.filesystem.newFileData(str, leaf(self._name)), n end
    return str, n
end

function File:write(data, size)
    if self._mode ~= 'w' and self._mode ~= 'a' then return false, "File " .. self._name .. " not opened for writing" end
    if type(data) ~= 'string' then data = data:getString() end
    if type(size) == 'number' then data = data:sub(1, size) end
    self._buf = self._buf .. data
    self._dirty = true
    return true
end

local function linesOf(data, onDone)
    local pos = 1
    return function()
        if pos > #data then
            if onDone then
                onDone(); onDone = nil
            end
            return nil
        end
        local nl = data:find("\n", pos, true)
        local line
        if nl then
            line = data:sub(pos, nl - 1); pos = nl + 1
        else
            line = data:sub(pos); pos = #data + 1
        end
        if line:sub(-1) == "\r" then line = line:sub(1, -2) end
        return line
    end
end

function File:lines()
    if self._mode ~= 'r' then error("File " .. self._name .. " is not opened for reading") end
    local rest = self._buf:sub(self._pos + 1)
    self._pos = #self._buf
    return linesOf(rest)
end

-----------------------------------------------------------------------------

function nativefs.newFile(name)
    if type(name) ~= 'string' then
        error("bad argument #1 to 'newFile' (string expected, got " .. type(name) .. ")")
    end
    return setmetatable({ _name = name, _mode = 'c', _buf = nil, _pos = 0, _bufferSize = 0, _bufferMode = 'none' }, File)
end

function nativefs.newFileData(filepath)
    local data, err = nxfs.read(resolve(filepath))
    if not data then return nil, err end
    return love.filesystem.newFileData(data, leaf(filepath))
end

function nativefs.mount(archive, mountPoint, appendToPath)
    local ok, res = pcall(love.filesystem.mount, resolve(archive), mountPoint, appendToPath)
    log(("mount(%s, %s) -> %s"):format(tostring(archive), tostring(mountPoint), tostring(ok and res)))
    return ok and res or false
end

function nativefs.unmount(archive)
    local ok, res = pcall(love.filesystem.unmount, resolve(archive))
    return ok and res or false
end

function nativefs.read(containerOrName, nameOrSize, sizeOrNil)
    local container, name, size
    if sizeOrNil then
        container, name, size = containerOrName, nameOrSize, sizeOrNil
    elseif not nameOrSize then
        container, name, size = 'string', containerOrName, 'all'
    else
        if type(nameOrSize) == 'number' or nameOrSize == 'all' then
            container, name, size = 'string', containerOrName, nameOrSize
        else
            container, name, size = containerOrName, nameOrSize, 'all'
        end
    end
    local data, err = nxfs.read(resolve(name))
    if not data then return nil, err end
    if type(size) == 'number' then data = data:sub(1, size) end
    if container == 'data' then return love.filesystem.newFileData(data, leaf(name)), #data end
    return data, #data
end

local function writeFile(mode, name, data, size)
    if type(data) ~= 'string' then data = data:getString() end
    if type(size) == 'number' then data = data:sub(1, size) end
    local ok, err = nxfs.write(resolve(name), data, mode)
    if not ok then return nil, err end
    return true
end

function nativefs.write(name, data, size) return writeFile('w', name, data, size) end

function nativefs.append(name, data, size) return writeFile('a', name, data, size) end

function nativefs.lines(name)
    local data, err = nxfs.read(resolve(name))
    if not data then return nil, err end
    return linesOf(data)
end

function nativefs.load(name)
    local chunk, err = nativefs.read(name)
    if not chunk then return nil, err end
    return loadstring(chunk, name)
end

function nativefs.getWorkingDirectory()
    return cwd
end

function nativefs.setWorkingDirectory(path)
    if type(path) ~= 'string' then return false, "Could not set working directory" end
    local target
    if splitMount(path) then
        target = normalize(path)
    elseif path:sub(1, 1) == "/" then
        target = normalize(SD_ROOT .. path)
    else
        target = normalize(cwd .. "/" .. path)
    end
    if target:sub(1, 3) ~= "sd:" then
        log(("setWorkingDirectory(%s) redirected to %s"):format(path, SD_ROOT))
        target = SD_ROOT
    end
    if nxfs.info(target) ~= "directory" then
        local ok, err = nativefs.createDirectory(target)
        if not ok then return false, "Could not set working directory: " .. tostring(err) end
    end
    cwd = target
    return true
end

function nativefs.getDriveList()
    return { "sd:/" }
end

function nativefs.createDirectory(path)
    local full = resolve(path)
    local mount, rest = splitMount(full)
    local current = (mount or "") .. "/"
    for seg in rest:gmatch("[^/]+") do
        current = current .. seg
        if nxfs.info(current) ~= "directory" then
            local ok, err = nxfs.mkdir(current)
            if not ok then return false, "Could not create directory " .. current .. ": " .. tostring(err) end
        end
        current = current .. "/"
    end
    return true
end

function nativefs.remove(name)
    local ok, err = nxfs.remove(resolve(name))
    if not ok then return false, "Could not remove " .. name .. ": " .. tostring(err) end
    return true
end

function nativefs.getDirectoryItems(dir)
    return nxfs.list(resolve(dir)) or {}
end

local function infoOf(full, filtertype)
    local t, size, modtime = nxfs.info(full)
    if not t then return nil end
    if type(filtertype) == 'string' and filtertype ~= t then return nil end
    return { type = t, size = size, modtime = modtime }
end

function nativefs.getDirectoryItemsInfo(path, filtertype)
    local items = {}
    local base = resolve(path)
    for _, name in ipairs(nxfs.list(base) or {}) do
        local info = infoOf(base .. "/" .. name, filtertype)
        if info then
            info.name = name
            items[#items + 1] = info
        end
    end
    return items
end

function nativefs.getInfo(path, filtertype)
    return infoOf(resolve(path), filtertype)
end

-- Steamodded's redirect layer
local redirects = {}

function nativefs.smodsAddRedirect(realPath, lfsPath)
    if redirects[realPath] then return false, 'A redirect with path "' .. realPath .. '" already exists' end
    redirects[realPath] = lfsPath
    return true
end

local function getRedirectPath(realPath)
    for p, r in pairs(redirects) do
        local len = #p
        local sub = realPath:sub(0, len + 1)
        if sub == p then return r end
        if sub == p .. "/" then return r .. "/" .. realPath:sub(len + 2) end
    end
    return false
end

local function patchSimple(n, l)
    return function(path, ...)
        local red = getRedirectPath(path)
        if red then return l(red, ...) end
        return n(path, ...)
    end
end

for _, v in ipairs { "newFile", "unmount", "write", "append", "lines", "load", "getDirectoryItems", "getInfo", "createDirectory", "remove", "mount", "newFileData" } do
    nativefs[v] = patchSimple(nativefs[v], love.filesystem[v])
end

do
    local getDirectoryItemsInfo = function(path, filtertype)
        local items = {}
        local files = love.filesystem.getDirectoryItems(path)
        for i = 1, #files do
            local filepath = string.format('%s/%s', path, files[i])
            local info = love.filesystem.getInfo(filepath, filtertype)
            if info then
                info.name = files[i]
                table.insert(items, info)
            end
        end
        return items
    end
    nativefs.getDirectoryItemsInfo = patchSimple(nativefs.getDirectoryItemsInfo, getDirectoryItemsInfo)
end

do
    local nRead = nativefs.read
    local lRead = love.filesystem.read
    function nativefs.read(containerOrName, nameOrSize, sizeOrNil)
        if sizeOrNil then
            local red = getRedirectPath(nameOrSize)
            if red then return lRead(containerOrName, red, sizeOrNil) end
            return nRead(containerOrName, nameOrSize, sizeOrNil)
        elseif not nameOrSize then
            local red = getRedirectPath(containerOrName)
            if red then return lRead(red, nameOrSize, sizeOrNil) end
            return nRead(containerOrName, nameOrSize, sizeOrNil)
        else
            if type(nameOrSize) == 'number' or nameOrSize == 'all' then
                local red = getRedirectPath(containerOrName)
                if red then return lRead(red, nameOrSize, sizeOrNil) end
                return nRead(containerOrName, nameOrSize, sizeOrNil)
            else
                local red = getRedirectPath(nameOrSize)
                if red then return lRead(containerOrName, red, sizeOrNil) end
                return nRead(containerOrName, nameOrSize, sizeOrNil)
            end
        end
    end
end

log("ready, cwd=" .. cwd)
return nativefs
