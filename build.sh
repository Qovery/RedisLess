#!/usr/bin/env sh

cargo build --release
cp target/release/libredisless.dylib clients/python/src/libredisless.dylib
