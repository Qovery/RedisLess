from os.path import dirname, abspath
from sys import platform

from cffi import FFI


class RedisLess(object):
    """
    RedisLess is a fast, lightweight, embedded and scalable in-memory Key/Value store library
    compatible with the Redis API.
    """

    def __init__(self, port: int = 16379):
        ffi = FFI()

        c_def = """
            // opaque pointer to Server struct
            typedef void* server;
            
            server redisless_server_new(unsigned short);
            void redisless_server_free(void* server);
            bool redisless_server_start(void* server);
            bool redisless_server_stop(void* server);
        """

        # ffi.set_source("c.binding", c_def)
        # ffi.compile(verbose=True)

        ffi.cdef(c_def)

        # support Windows / Linux and MacOSX - load the right lib
        lib_extension = None
        if platform.startswith('linux'):
            lib_extension = 'so'
        elif platform.startswith('darwin'):
            lib_extension = 'dylib'
        elif platform.startswith('win32'):
            lib_extension = 'dll'
        else:
            print('platform {} not supported'.format(platform))
            exit(1)

        source_path = dirname(abspath(__file__))
        self._C = ffi.dlopen("{}/libredisless.{}".format(source_path, lib_extension))
        self._redisless_server = self._C.redisless_server_new(port)

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
