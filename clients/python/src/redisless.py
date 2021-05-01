from os.path import dirname, abspath

from cffi import FFI


class RedisLess(object):
    """
    RedisLess is a fast, lightweight, embedded and scalable in-memory Key/Value store library
    compatible with the Redis API.
    """

    def __init__(self, port: int = 16379):
        ffi = FFI()

        c_def = """
            // opaque pointer to RedisLess struct
            typedef void* redisless;
            // opaque pointer to Server struct
            typedef void* server;
            
            redisless redisless_new();
            server redisless_server_new(redisless, unsigned short);
            void redisless_server_free(void* redisless);
            bool redisless_server_start(void* server);
            bool redisless_server_stop(void* server);
        """

        # ffi.set_source("c.binding", c_def)
        # ffi.compile(verbose=True)

        ffi.cdef(c_def)

        # TODO support Windows / Linux and MacOSX - load the right lib
        source_path = dirname(abspath(__file__))
        self._C = ffi.dlopen("{}/libredisless.dylib".format(source_path))
        self._redisless = self._C.redisless_new()
        self._redisless_server = self._C.redisless_server_new(self._redisless, port)

    # TODO implement destructor and free redisless

    def start(self) -> bool:
        """
        Start local embedded RedisLess instance
        :return: true if RedisLess instance has been started correctly; false otherwise
        """
        return self._C.redisless_server_start(self._redisless_server)

    def stop(self) -> bool:
        """
        Stop local embedded RedisLess instance
        :return: true if RedisLess instance has been stopped correctly; false otherwise
        """
        return self._C.redisless_server_stop(self._redisless_server)
