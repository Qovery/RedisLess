#!/bin/sh

server_source="/tmp/server.c"
redis_source="https://raw.githubusercontent.com/redis/redis/unstable/src/server.c"
supported="/tmp/supported"
redisless_source="https://raw.githubusercontent.com/Qovery/RedisLess/main/redisless/src/command/mod.rs"
rediscommands=$"/tmp/rediscommands"
redisless_cmds=$"/tmp/redisless"

wget -O $server_source $redis_source
wget -O $redisless_cmds $redisless_source
cat $redisless_cmds | grep -E "(.[ ])b\".*$" | sed -E 's/.*b"([a-z]*)".*$/\1/' > $supported
cat $server_source | grep -E '^    \{"' | sed -E 's/^    \{"(.*)".*$/\1/' | awk '{print $0}' > $rediscommands
join <(sort -u $rediscommands) <(sort -u $supported) | awk '{print "- [x] " $0}' > commands.md
comm -23 <(sort -u $rediscommands) <(sort -u $supported) | awk '{print "- [ ] " $0}' >> commands.md
rm $rediscommands
rm $server_source
rm $supported
