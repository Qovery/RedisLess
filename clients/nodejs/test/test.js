const {RedisLess} = require('../lib/index');
const {strictEqual} = require('assert');
const redis = require('async-redis');

jest.setTimeout(1000 * 60);

test('exec set get delete commands', async (done) => {
    const port = 16379;
    const redisless = new RedisLess(port);

    strictEqual(redisless.start(), true);

    const r = redis.createClient({host: 'localhost', port: port});

    strictEqual(await r.get('key'), null);

    await r.set('key', 'value');
    strictEqual(await r.get('key'), 'value');

    strictEqual(await r.del('key'), 1);
    strictEqual(await r.get('key'), null);
    strictEqual(await r.del('key'), 0);

    strictEqual(redisless.stop(), true);

    done();
});
