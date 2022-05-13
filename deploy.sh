#/bin/sh

set -xe

if [ -f ./stechuhr.tar ]; then
    rm ./stechuhr.tar
fi

cargo build && cargo build --release
tar cf stechuhr.tar -T ./deploylist.txt