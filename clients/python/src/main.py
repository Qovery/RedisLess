#!/usr/bin/env python
import redis
import time

from redisless import RedisLess

if __name__ == '__main__':

    port = 16379
    redisless = RedisLess(port=port)
    assert redisless.start()

    redis = redis.Redis(host='127.0.0.1', port=port)

    for _ in range(100):

        assert redis.ping()

        assert redis.set('key', 'value')
        assert redis.get('key') == b'value'

        assert redis.get('key2') is None
        assert redis.get('not existing key') is None

        assert redis.set('key2', 'valueA')
        assert redis.getset('key2', 'valueB') == b'valueA'

        duration = 1
        assert redis.expire('key1000', 10) == False
        assert redis.expire('key2', duration)
        assert redis.get('key2') == b'valueB'
        time.sleep(duration)
        assert redis.get('key2') is None

        assert redis.getset('key2', 'valueC') is None
        assert redis.delete('key2')

        assert redis.delete('key') == 1

        # Uncomment when incrby has been implemented
        # The redis python library calls incr as incrby

        # let increment = 5
        # assert redis.set('key3', 30) == True
        # assert redis.incr('key3') == 35
        # assert redis.get('key3') == 40

    assert redisless.stop()
