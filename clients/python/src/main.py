#!/usr/bin/env python
import time

from clients.python.src.redisless import RedisLess

if __name__ == '__main__':
    redisless = RedisLess()
    redisless.start_server()
    redisless.stop_server()
    time.sleep(60)
