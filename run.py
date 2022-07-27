#!/usr/bin/env python
import subprocess
import sys
from functools import partial
import numpy as np

CARGO_RUN = "cargo run --release -p benchmarks --".split(" ")
DRY_RUN = True
WARMUP = "./warmup/v2"
RESULT = "./paper_experiment/oakland"


def to_amt_size(key):
    if key == "fresh":
        base = 1e6
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
    prefix = CARGO_RUN + ["--no-stat", "--warmup-to", WARMUP]
    run("rm -rf __benchmarks")
    if shards is None:
        run(prefix + f"-a {alg} -k {key}".split(" "))
    else:
        run(prefix + f"-a {alg} -k {key} --shard-size {shards}".split(" "))


def bench(task, alg, key, shards=None, low_memory=False):
    if alg == "samt":
        amt_size = to_amt_size(key)
        if amt_size > 26:
            return
        alg = alg + f"{amt_size:d}"

    prefix = CARGO_RUN + f"--max-time 3600 --max-epoch 10000 -a {alg}".split(" ")

    if task == "time":
        prefix = prefix + ["--no-stat"]
    else:
        pass

    if key != "fresh":
        prefix = prefix + f"--warmup-from {WARMUP} -k {key}".split(" ")
    else:
        prefix = prefix + f"--no-warmup -k 10g".split(" ")

    if low_memory:
        if task == "stat":
            return
        prefix = prefix + "--cache-size 800".split(" ")
    else:
        prefix = prefix + "--cache-size 1500".split(" ")

    run("rm -rf __benchmarks")

    if shards is None:
        output = f"{RESULT}/{task}_{alg}_{key}.log"
        run(prefix, output)
    else:
        if low_memory:
            output = f"{RESULT}/{task}_{alg}{shards}_{key}_lowmem.log"
        else:
            output = f"{RESULT}/{task}_{alg}{shards}_{key}.log"
        run(prefix + f"--shard-size {shards}".split(" "), output)


bench_time = partial(bench, "time")
bench_stat = partial(bench, "stat")


def run_all(run_one):
    for key in ["1m", "10m", "100m", "fresh"]:
        # run_one("amt", key)
        # run_one("mpt", key)
        run_one("samt", key)
        for shards in [1]:
            # run_one("amt", key, shards, low_memory=True)
            # run_one("amt", key, shards)
            pass


if __name__ == "__main__":
    run("rm -rf __reports __benchmarks")
    # run("mkdir -p ./warmup/v2")
    run(f"mkdir -p {RESULT}")

    # run_all(warmup)
    run_all(bench_time)
    # run_all(bench_stat)
