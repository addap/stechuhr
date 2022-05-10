#!/bin/sh

set -xe 

if [ -f "stechuhr.sqlite3" ]; then
    rm stechuhr.sqlite3
fi

cat migrations/**/up.sql | sqlite3 stechuhr.sqlite3


cat > .env << EOF
DATABASE_URL="./stechuhr.sqlite3"
RUST_LOG=error
EOF