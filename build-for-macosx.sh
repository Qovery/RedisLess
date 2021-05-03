#!/usr/bin/env sh

# MacOSX dependencies:
# 1. Install Xcode
# 2. Install Rust - https://www.rust-lang.org/tools/install
# 3. Run `brew install mingw-w64`
# 4 Run `brew install FiloSottile/musl-cross/musl-cross`
# 5 Run `brew tap SergioBenitez/osxct && brew install x86_64-unknown-linux-gnu`

rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-apple-darwin

# Let's build
cd redisless && cargo clean && cargo build --release

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  # Linux
  echo "Platform Linux detected"
  RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc" cargo build --target x86_64-pc-windows-gnu --release
elif [[ "$OSTYPE" == "darwin"* ]]; then
  # Mac OSX
  echo "Platform MacOSX detected"
  RUSTFLAGS="-C linker=x86_64-apple-darwin14-clang" cargo build --target x86_64-apple-darwin --release
  RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc" cargo build --target x86_64-pc-windows-gnu --release
  RUSTFLAGS="-C linker=x86_64-unknown-linux-gnu-gcc" cargo build --target x86_64-unknown-linux-gnu --release
elif [[ "$OSTYPE" == "win32" ]]; then
  # Windows
  echo "Platform Windows detected"
  RUSTFLAGS="-C linker=x86_64-unknown-linux-gnu-gcc" cargo build --target x86_64-unknown-linux-gnu --release
else
  echo "Platform not supported"
  exit 1
fi

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
cp redisless/target/x86_64-unknown-linux-gnu/release/libredisless.so clients/python/src/libredisless.so

# Linux NodeJS
cp redisless/target/x86_64-unknown-linux-gnu/release/libredisless.so clients/nodejs/lib/libredisless.so

# Linux Golang
cp redisless/target/x86_64-unknown-linux-gnu/release/libredisless.so clients/golang/lib/libredisless.so
