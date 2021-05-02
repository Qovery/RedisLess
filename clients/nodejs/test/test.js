const {RedisLess} = require('../lib/index');
const {strictEqual} = require('assert');
const redis = require('async-redis');

test('exec set get delete commands', async () => {
    const port = 16379;
    const redisless = new RedisLess(port);

    strictEqual(redisless.start(), true);

    const r = redis.createClient({host: 'localhost', port: port});
    const y = await r.get('key');

    r.set('key', 'value');
    const x = await r.get('key');
    strictEqual(x, 'value');

    strictEqual(redisless.stop(), true);
});
