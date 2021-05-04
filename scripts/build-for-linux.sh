#!/usr/bin/env sh

rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin

# Let's build
cd redisless

cargo clean

# build for Linux
cargo build --release

# cross build for MacOSX
#RUSTFLAGS="-C linker=x86_64-apple-darwin14-clang" cargo build --target x86_64-apple-darwin --release
cargo build --target x86_64-apple-darwin --release

# cross build for Windows
RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc" cargo build --target x86_64-pc-windows-gnu --release

cd ..

# MacOSX Python
cp redisless/target/x86_64-apple-darwin/release/libredisless.dylib clients/python/src/libredisless.dylib

# MacOSX NodeJS
cp redisless/target/x86_64-apple-darwin/release/libredisless.dylib clients/nodejs/lib/libredisless.dylib

# MacOSX Golang
cp redisless/target/x86_64-apple-darwin/release/libredisless.dylib clients/golang/lib/libredisless.dylib

# Windows Python
cp redisless/target/x86_64-pc-windows-gnu/release/redisless.dll clients/python/src/libredisless.dll

# Windows NodeJS
cp redisless/target/x86_64-pc-windows-gnu/release/redisless.dll clients/nodejs/lib/libredisless.dll

# Windows Golang
cp redisless/target/x86_64-pc-windows-gnu/release/redisless.dll clients/golang/lib/libredisless.dll

# Linux Python
cp redisless/target/release/libredisless.so clients/python/src/libredisless.so

# Linux NodeJS
cp redisless/target/release/libredisless.so clients/nodejs/lib/libredisless.so

# Linux Golang
cp redisless/target/release/libredisless.so clients/golang/lib/libredisless.so
