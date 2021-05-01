const path = require('path');
const ffi = require('ffi-napi');

const libPath = path.join(__dirname, 'libredisless.dylib');

const libm = ffi.Library(libPath, {
    'redisless_server_new': ['void*', ['short']],
    'redisless_server_start': ['bool', ['void*']],
    'redisless_server_stop': ['bool', ['void*']],
});

function RedisLess(port = 16379) {
    this.server = libm.redisless_server_new(port);
    return this;
}

RedisLess.prototype.start = function () {
    return libm.redisless_server_start(this.server);
};

RedisLess.prototype.stop = function () {
    return libm.redisless_server_stop(this.server);
};

exports.RedisLess = RedisLess;
