#!/bin/sh

set -xe 

if [ -f "stechuhr.sqlite3" ]; then
    rm stechuhr.sqlite3
fi

if [ -f "stechuhr.sqlite3.template" ]; then
    cp stechuhr.sqlite3.template stechuhr.sqlite3
else
    cat migrations/**/up.sql | sqlite3 stechuhr.sqlite3
    cargo run --bin add_6am_events

    cp stechuhr.sqlite3 stechuhr.sqlite3.template
fi

mkdir -p auswertung

cat > .env << EOF
DATABASE_URL="./stechuhr.sqlite3"
RUST_LOG=error
WGPU_BACKEND=gl
EOF