#!/bin/bash

rm -rf __reports __benchmarks

for keys in 100k 1m 10m 100m 1g
do
    for type in raw amt mpt
    do 
        rm -rf __benchmarks
        cargo run --release -p benchmarks -- -a $type -k $keys --max-time 3600 --max-epoch 100000 --profile-epoch 10000 --report-to ./__reports --no-stat
    done
done

mv __reports "results/$(date +'%Y%m%d%H%M')"
