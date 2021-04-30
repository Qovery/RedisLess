#!/usr/bin/env python
import redis

from redisless import RedisLess

if __name__ == '__main__':
    redisless = RedisLess()
    redisless.start()

    redis = redis.Redis(host='127.0.0.1', port=16379, db=0)
    # redis = redis.Redis(host='127.0.0.1', port=3333, db=0)
    redis.get('key2')
    redis.set('key', 'value')

    v = redis.get('key')
    assert v == b'value'

    v = redis.get('not existing key')
    assert v is None
    redis.delete('key')
    redis.close()

    redisless.stop()
