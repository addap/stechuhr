#/bin/sh

set -xe

if [ -f ./stechuhr.tar ]; then
    rm ./stechuhr.tar
fi
tar cf stechuhr.tar -T ./deploylist.txt