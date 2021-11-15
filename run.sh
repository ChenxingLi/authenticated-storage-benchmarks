#!/bin/bash

rm -rf __reports __benchmarks

for keys in 10k 100k 1m 10m 100m
do
    for type in raw amt mpt dmpt
    do 
        rm -rf __benchmarks
        cargo run --release -p benchmarks -- -a $type -k $keys --report-to ./__reports
    done
done

mv __reports "results/$(date +'%Y%m%d%H%M')"
