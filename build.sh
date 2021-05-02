#!/usr/bin/env sh

cd redisless && cargo build --release && cd ..

cp redisless/target/release/libredisless.dylib clients/python/src/libredisless.dylib
cp redisless/target/release/libredisless.dylib clients/nodejs/lib/libredisless.dylib
