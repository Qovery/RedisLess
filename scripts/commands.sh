#!/bin/sh

redis_source="https://raw.githubusercontent.com/redis/redis/unstable/src/server.c"
redisless_source="https://raw.githubusercontent.com/Qovery/RedisLess/main/redisless/src/command/mod.rs"
rediscommands=$"/tmp/rediscommands"
supported="/tmp/supported"

curl -s $redisless_source | grep -E "(.[ ])b\".*$" | sed -E 's/.*b"([a-z]*)".*$/\1/' > $supported
curl -s $redis_source| grep -E '^    \{"' | sed -E 's/^    \{"(.*)".*$/\1/' | awk '{print $0}' > $rediscommands
join <(sort -u $rediscommands) <(sort -u $supported) | awk '{print "- [x] " $0}' > commands.md
comm -23 <(sort -u $rediscommands) <(sort -u $supported) | awk '{print "- [ ] " $0}' >> commands.md
rm $supported
rm $rediscommands
cat commands.md
