RedisLess
===========

[![Discord](https://img.shields.io/discord/688766934917185556?label=discord)](https://discord.qovery.com) [![Test and Build](https://github.com/Qovery/RedisLess/workflows/Test%20and%20Build/badge.svg?branch=main)](https://github.com/Qovery/RedisLess/actions?query=workflow%3A%22Test+and+Build%22)

---------

**THIS PROJECT IS TESTABLE, BUT NOT PRODUCTION READY YET!!**

---------

**RedisLess is a fast, lightweight, embedded and scalable in-memory Key/Value store library compatible with
the [Redis](https://redis.io/topics/introduction) API.**

> RedisLess is the concatenation of Redis and Serverless.

# The starting point

[Romaric](https://twitter.com/rophilogene), initiator of RedisLess:

> I am a backend engineer. And as a backend developer, I mostly work on providing web API for frontend developers and backend developers. Redis is part of my toolset - especially when I share data and sync states between my backend apps. One of the downsides of Redis (like for all databases) is that you need to spawn a Redis server and maintain it.

> So, imagine a world where your Redis instance is nothing more than a lib in your app. Imagine that this lib can sync the data with other neighborhood instances?

> That's the idea behind RedisLess, a lightweight, embedded, and scalable in-memory Key/Value store library compatible with the Redis API.

Read more about RedisLess:
* Apr 28 2021: The RedisLess project was initially announced [here](https://www.heapstack.sh/redisless-blazingly-fast-serverless-redis).
* May 03 2021: [Interesting discussion about RedisLess](https://www.reddit.com/r/rust/comments/n3d1zw/i_am_building_a_serverless_version_of_redis/) on Reddit.
* May 13 2021: [How I imagine the future of databases in the Cloud](https://www.heapstack.sh/how-i-imagine-the-future-of-databases-in-the-cloud)

# How to use it?

To use RedisLess, you only need to:

1. Install RedisLess library for your favorite language (see supported clients below).
2. Connect your favorite Redis client to `redis://localhost:16379`.
3. You don't need to change your code - RedisLess is Redis API compatible. (see [supported Redis commands](REDIS_FEATURES.md))

Under the hood, the RedisLess library starts a local Redis API compatible instance on port `16739` (you can change the port).

## NodeJS client

### Install

```bash
# RedisLess library with Python binding
npm install redisless

# redis client
npm install async-redis
```

### Usage

```js
const {RedisLess} = require('redisless');
const redis = require('async-redis');

const port = 16379;
const redisless = new RedisLess(port);

redisless.start();

const r = redis.createClient({host: 'localhost', port: port});

await r.set('foo', 'bar');
await r.get('foo'); // return 'bar'
await r.del('foo'); // delete key 'foo'

redisless.stop();
```

## Python client

### Install

```bash
# RedisLess library with Python binding
pip install redisless

# redis client
pip install redis
```

### Usage

```python
from redisless import RedisLess
import redis

redisless = RedisLess()

# start RedisLess embedded instance
redisless.start()

# Connect to RedisLess on localhost:16379
redis = redis.Redis(host='localhost', port=16379)

redis.set('foo', 'bar')
redis.get('foo')  # return bar 
redis.delete('foo')  # return 1 

# stop RedisLess embedded instance
redisless.stop()
```


# What is RedisLess

* Embedded in-memory. (No Redis server required!).
* Compatible with the Redis API (RESP).
* Not intrusive: you don't need to modify you code to use RedisLess!
* Built with DX and performance in mind.
* Cloud native friendly.

What RedisLess is not:

* Production ready yet.
* 1:1 Redis implementation.

# Planned features

- [ ] Redis API ([see implemented features](REDIS_FEATURES.md))
- [ ] Cluster mode
- [ ] Auto-discovery
- [ ] Disk persistence

# Planned supported clients

- [x] [NodeJS](clients/nodejs)
- [x] [Golang](clients/golang)
- [x] [Python](clients/python)
- [ ] Java

# Expected performance

Strong attention to performance and code cleanliness is given when designing RedisLess. It aims to be crash-free, super-fast and put a
minimum strain on your server resources (benchmarks will come soon).

RedisLess is written in Rust and export functions through FFI (Foreign Function Interface), making it usable from any language. We provide
clients for NodeJS, Python, Golang, Java, and Rust. Supporting a new language can be done in 5 minutes. Look at [Python](clients/python)
and [NodeJS](clients/nodejs) clients implementation for inspiration.

# Contribution welcome!

It is never too soon to contribute to a great project. If you are interested in contributing, please join us
on [Discord](https://discord.qovery.com), then we can discuss. The project is in its early days, but we are serious about building a solid
library to help thousands of developers.

## Set up MacOSX development environment

### Pre-requisites
1. Install [Xcode](https://developer.apple.com/xcode/)
2. Install [Brew](https://brew.sh/)   
3. Install [Rust](https://www.rust-lang.org/tools/install)
4. Run `brew install mingw-w64` to cross build for Windows
5. Run `brew install FiloSottile/musl-cross/musl-cross` to cross build for Linux 
6. Run `brew tap SergioBenitez/osxct && brew install x86_64-unknown-linux-gnu` # to cross build for Linux

### Build clients

To build NodeJS, Python, Golang and other libs you need to run:

```bash
cd scripts && ./build-for-mac.sh && cd ..
```

### Use clients

Once the libs built, you are ready to use the clients into the [clients](clients) folder.

# Contributors

Thanks to our contributors ❤️

<a href="https://github.com/Qovery/RedisLess/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=Qovery/RedisLess" />
</a>

# References

- Redis Internals: [Free e-book](https://redislabs.com/ebook)
- [Redis Protocol Specification](https://redis.io/topics/protocol) (RESP)
- [Sonic](https://github.com/valeriansaliou/sonic): fast and lightweight Elasticsearch alternative
- [Mnesia](https://erlang.org/doc/man/mnesia.html): Erlang distributed DBMS
- [Hazelcast](https://hazelcast.com): Java distributed in-memory datastructures
