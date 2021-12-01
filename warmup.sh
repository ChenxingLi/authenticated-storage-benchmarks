#!/bin/bash
rm -rf __reports __benchmarks

for keys in 100k 1m 10m 100m
do
    for type in amt
    do
        rm -rf __benchmarks
        cargo run --release -p benchmarks -- -a $type -k $keys --max-time 0 --warmup-to ./warmup/v0 --no-stat
    done
done