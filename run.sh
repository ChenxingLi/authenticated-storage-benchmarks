#!/bin/bash

rm -rf __reports __benchmarks

for keys in 10k 100k 1m 10m 100m
do
    for type in 1 2 3 4
    do 
        cargo run --release -p benchmarks -- $type $keys 
    done
done

mv __reports "results/$(date +'%Y%m%d-%H%M')"
