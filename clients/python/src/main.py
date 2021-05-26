#!/usr/bin/env python
import redis

from redisless import RedisLess

if __name__ == '__main__':
    port = 16379

    redisless = RedisLess(port=port)
    assert redisless.start()

    redis = redis.Redis(host='127.0.0.1', port=port)

    for _ in range(20):
        assert redis.ping()

        redis.set('n', 8)
        assert redis.incr('n') == 9
        assert redis.decr('n') == 8

        redis.set('key', 'value')
        assert redis.get('key') == b'value'
        assert redis.type('key') == b'string'

        assert redis.get('key2') is None
        assert redis.get('not existing key') is None

        assert redis.delete('key') == 1

    assert redisless.stop()
