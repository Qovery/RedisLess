package redisless

/*
#cgo LDFLAGS: ./lib/libredisless.dylib
#include <stdbool.h>
            typedef void* server;
            server redisless_server_new(unsigned short);
            void redisless_server_free(void* server);
            bool redisless_server_start(void* server);
            bool redisless_server_stop(void* server);
*/
import "C"
import (
	"unsafe"
)

type RedisLess C.server

func NewRedisLess(port int) RedisLess {
	return RedisLess(C.redisless_server_new(C.ushort(port)))
}

func Start(r RedisLess) bool {
	return bool(C.redisless_server_start(unsafe.Pointer(r)))
}

func Stop(r RedisLess) bool {
	return bool(C.redisless_server_stop(unsafe.Pointer(r)))
}
