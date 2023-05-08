#!/usr/bin/env python3
import subprocess
import sys
from functools import partial
import numpy as np

CARGO_RUN = "cargo run --release --".split(" ")
DRY_RUN = False
WARMUP = "./warmup/v4"
RESULT = "./paper_experiment/osdi23"


def to_amt_size(key):
    if key == "fresh":
        return 1e8
    if key == "real":
        return 2e6
    elif key[-1].lower() in "kmg":
        exp = 10 ** ("kmg".index(key[-1].lower()) * 3 + 3)
        base = float(key[:-1]) * exp
    else:
        base = float(key)
    return int(np.ceil(np.log2(base * 5)))


def run(commands, output=None):
    if type(commands) is str:
        commands = commands.split(" ")

    if output is None:
        message = " ".join(commands)
    else:
        message = " ".join(commands) + f" > {output}"

    if DRY_RUN:
        print(message)
        return

    print("")
    print(f">>>>>>>>>>> {message}")
    sys.stdout.flush()

    if output is not None:
        output = open(output, "w")

    subprocess.run(commands, stdout=output)
    print(f"<<<<<<<<<<< done")
    sys.stdout.flush()


def warmup(alg, key, shards=None):
    if key == "fresh":
        return

    if alg == "amt":
        amt_size = to_amt_size(key)
        if amt_size > 26:
            return
        alg = alg + f"{amt_size:d}"

    prefix = CARGO_RUN + ["--no-stat", "--warmup-to", WARMUP]
    run("rm -rf __benchmarks")

    if key == "real":
        prefix = prefix + ["--real-trace"]
    else:
        prefix = prefix + f"-k {key}".split(" ")

    if shards is None:
        run(prefix + f"-a {alg}".split(" "))
    else:
        run(prefix + f"-a {alg} --shards {shards}".split(" "))


def bench(task, alg, key, shards=None, low_memory=False, high_memory= 0):
    if alg == "amt":
        amt_size = to_amt_size(key)
        if amt_size > 26:
            return
        alg = alg + f"{amt_size:d}"

    prefix = CARGO_RUN + f"--max-time 3600 -a {alg}".split(" ")

    if task == "time":
        prefix = prefix + ["--no-stat"]
    else:
        pass

    if key != "real":
        prefix = prefix + "--max-epoch 200".split(" ")

    if key == "fresh":
        prefix = prefix + ["--no-warmup"]
    else:
        prefix = prefix + f"--warmup-from {WARMUP}".split(" ")

    if key == "fresh":
        prefix = prefix + f"-k 10g".split(" ")
    elif key == "real":
        prefix = prefix + f"--real-trace --report-epoch 25".split(" ")
    else:
        prefix = prefix + f"-k {key}".split(" ")

    if low_memory:
        if task == "stat":
            return
        prefix = prefix + "--cache-size 800".split(" ")
    elif high_memory==1:
        if task == "stat":
            return
        prefix = prefix + "--cache-size 4096".split(" ")
    elif high_memory==2:
        if task == "stat":
            return
        prefix = prefix + "--cache-size 8192".split(" ")
    else:
        prefix = prefix + "--cache-size 1500".split(" ")

    run("rm -rf __benchmarks")

    suffix = ""
    if low_memory:
        suffix = "_lowmem"
    elif high_memory > 0:
        suffix = f"_highmem{high_memory}"

    if shards is None:
        output = f"{RESULT}/{task}_{alg}_{key}{suffix}.log"
        run(prefix, output)
    else:
        output = f"{RESULT}/{task}_{alg}{shards}_{key}{suffix}.log"
        run(prefix + f"--shards {shards}".split(" "), output)


bench_time = partial(bench, "time")
bench_stat = partial(bench, "stat")


def warmup_all():
    for key in ["1m", "10m", "100m", "real"]:
        warmup("raw", key)
        warmup("lvmt", key)
        # warmup("rain", key)
        warmup("mpt", key)
        for shards in [64, 16]:
            warmup("lvmt", key, shards)

def run_all(run_one):
    for key in ["1m", "10m", "fresh", "real", "100m"]:
        run_one("raw", key)
        run_one("lvmt", key)
        # run_one("rain", key)
        run_one("mpt", key)
        # run_one("mpt", key, high_memory=1)
        # run_one("mpt", key, high_memory=2)
        for shards in [64, 16]:
            run_one("lvmt", key, shards)
        run_one("lvmt", key, 16, low_memory=True)



if __name__ == "__main__":
    run("rm -rf __reports __benchmarks")
    run(f"mkdir -p {WARMUP}")
    run(f"mkdir -p {RESULT}")

    warmup_all()
    run_all(bench_time)
    run_all(bench_stat)
