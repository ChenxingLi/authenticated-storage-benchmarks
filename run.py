#!/usr/bin/env python
import subprocess
from functools import partial

CARGO_RUN = "cargo run --release -p benchmarks --".split(" ")
DRY_RUN = False


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

    if output is not None:
        output = open(output, "w")

    subprocess.run(commands, stdout=output)
    print(f"<<<<<<<<<<< done")


def warmup(alg, key, shards=None):
    if key == "fresh":
        return
    prefix = CARGO_RUN + ["--no-stat", "--warmup-to", "./warmup/v1/"]
    run("rm -rf __benchmarks")
    if shards is None:
        run(prefix + f"-a {alg} -k {key}".split(" "))
    else:
        run(prefix + f"-a {alg} -k {key} --shard-size {shards}".split(" "))


def bench(task, alg, key, shards=None):
    run("rm -rf __benchmarks")

    prefix = CARGO_RUN + f"--max-time 3600 --max-epoch 10000 -a {alg}".split(" ")

    if task == "time":
        prefix = prefix + ["--no-stat"]
    else:
        pass

    if key != "fresh":
        prefix = prefix + f"--warmup-from ./warmup/v1/ -k {key}".split(" ")
    else:
        prefix = prefix + f"--no-warmup -k 10g".split(" ")

    if shards is None:
        output = f"paper_experiment/{task}/{alg}_{key}.log"
        run(prefix, output)
    else:
        output = f"paper_experiment/{task}/{alg}{shards}_{key}.log"
        run(prefix + f"--shard-size {shards}".split(" "), output)


bench_time = partial(bench, "time")
bench_stat = partial(bench, "stat")


def run_all(run_one):
    for key in ["1m", "10m", "100m", "fresh"]:
        run_one("amt", key)
        run_one("mpt", key)
        for shards in [64, 16]:
            run_one("amt", key, shards)


if __name__ == "__main__":
    run("rm -rf __reports __benchmarks")
    run("mkdir -p ./warmup/v1")
    run("mkdir -p ./paper_experiment/time")
    run("mkdir -p ./paper_experiment/stat")

    warmup("amt", "100m", 16)
    warmup("mpt", "100m")
    # run_all(warmup)
    # run_all(bench_time)
    # run_all(bench_stat)
