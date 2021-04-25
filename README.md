# RedisLess
RedisLess is an embedded and scalable in-memory Redis-like library that act as a Redis Server and is compatible with the Redis API.

> RedisLess is the concatenation of Redis + Serverless.

## Why
Developers often use Redis to share data and states between apps. The problem is that we need to set up a Redis server and manage it. Worst, scaling out Redis on multiple nodes is a nightmare. RedisLess brings you the ease of use of Redis without the inconvenience.

What RedisLess is:

* Embedded in-memory. (No Redis server required!).
* Built with DX and performance in mind.
* Compatible with the Redis API (RESP).

What RedisLess is not:

* Production ready yet.
* 1:1 Redis implementation.

# Usage

## Library
## Python

### Install
```bash
pip install redisless # RedisLess library with Python binding
pip install redis # redis client
```

### Usage
```python
from redisless import RedisLess
import redis

redisless = RedisLess()

# start RedisLess embedded local server
redisless.start_server()

# Connect to RedisLess on localhost:16379
redis = redis.Redis(host='localhost', port=16379, db=0)

redis.set('foo', 'bar')
redis.get('foo') # return bar 

# stop RedisLess embedded local server
redisless.stop_server()
```

# Features
- [ ] Compatible with the Redis API ([see details](REDIS_FEATURES.md))
- [ ] Auto-discovery
- [ ] Atomic operations
- [ ] Multi-threaded
- [ ] Memory off-load and disk persistence
- [ ] Cluster mode with replication (RAFT)

# Supported clients
- [ ] NodeJS
- [ ] Golang
- [ ] Python
- [ ] Java

# Performance
TODO

# Contribution welcome!
TODO

# References

- Redis Internals: [Free e-book](https://redislabs.com/ebook)
- [Redis Protocol Specification](https://redis.io/topics/protocol) (RESP)
- [Sonic](https://github.com/valeriansaliou/sonic): fast and lightweight Elasticsearch alternative
