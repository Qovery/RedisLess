package redisless

import (
	"context"
	"github.com/go-redis/redis/v8"
	"github.com/stretchr/testify/assert"
	"strconv"
	"testing"
)

var ctx = context.Background()

func TestExecSetGetDelCommands(t *testing.T) {
	port := 12345
	redisLess := NewRedisLess(port)
	assert.NotNil(t, redisLess)

	started := Start(redisLess)
	assert.True(t, started)

	client := redis.NewClient(&redis.Options{
		Addr:     "localhost:" + strconv.Itoa(port),
		Password: "",
		DB:       0,
	})

	key := "key"
	value := "value"

	err := client.Set(ctx, key, value, 0).Err()
	assert.Nil(t, err)

	valueInRedisLess, err := client.Get(ctx, key).Result()
	assert.Nil(t, err)
	assert.Equal(t, value, valueInRedisLess)

	i, err := client.Del(ctx, key).Result()
	assert.Nil(t, err)
	assert.EqualValues(t, 1, i)

	valueInRedisLess, err = client.Get(ctx, key).Result()
	assert.Empty(t, valueInRedisLess)
	assert.Equal(t, redis.Nil, err)

	stopped := Stop(redisLess)
	assert.True(t, stopped)
}
