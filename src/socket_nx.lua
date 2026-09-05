-- Minimal LuaSocket
--
-- The Switch LÖVE build ships without LuaSocket. Steamodded only uses it to reach its
-- optional debug console, so every operation here just fails cleanly.

local socket = { _VERSION = "LuaSocket stub (lovely-nx)" }

local function fail() return nil, "sockets are not available on Switch" end

local Client = {}
Client.__index = Client
function Client:settimeout() return true end

function Client:setoption() return true end

function Client:connect() return fail() end

function Client:bind() return fail() end

function Client:listen() return fail() end

function Client:accept() return fail() end

function Client:send() return fail() end

function Client:receive() return fail() end

function Client:close() return true end

function Client:getsockname() return "0.0.0.0", 0 end

function Client:getpeername() return fail() end

function socket.tcp() return setmetatable({}, Client) end

function socket.tcp4() return setmetatable({}, Client) end

function socket.tcp6() return setmetatable({}, Client) end

function socket.udp() return setmetatable({}, Client) end

function socket.connect() return fail() end

function socket.bind() return fail() end

function socket.select() return {}, {}, "timeout" end

function socket.sleep(t) if love and love.timer then love.timer.sleep(t) end end

function socket.gettime()
    if love and love.timer then return love.timer.getTime() end
    return os.time()
end

function socket.dns() return fail() end

socket.dns = { toip = fail, tohostname = fail, gethostname = function() return "switch" end }

return socket
