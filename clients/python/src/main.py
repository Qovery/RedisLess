#!/usr/bin/env python
import time

import redis
from clients.python.src.redisless import RedisLess

if __name__ == '__main__':
    redisless = RedisLess()
    redisless.start_server()

    redis = redis.Redis(host='127.0.0.1', port=16379, db=0)
    redis.get('key2')
    redis.set('key', 'value')
    assert redis.get('key') == 'value'
    assert redis.get('not existing key') is None
    redis.delete('key')

    redisless.stop_server()
