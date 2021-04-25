#!/usr/bin/env python
from cffi import FFI


if __name__ == '__main__':
    ffi = FFI()
    ffi.cdef("""
        typedef void* redisless;
        
        redisless redisless_new();
        
        void redisless_set(redisless, char *key, char *value);
        *const char redisless_get(redisless, char *key);
    """)

    C = ffi.dlopen('../../../target/release/libredisless.dylib')

    redisless = C.redisless_new()
    print(redisless)
    C.redisless_set(redisless, b'key', b'value')
    # print(redisless.get('key'))

