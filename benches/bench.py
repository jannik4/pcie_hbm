import subprocess
import json
import sys

TRANSFER_SIZE = 1024 * 1024 * 1024  # 1GiB
BLOCK_SIZES = [
    16 * 1024,  # 16KiB
    32 * 1024,  # 32KiB
    64 * 1024,  # 64KiB
    128 * 1024,  # 128KiB
    256 * 1024,  # 256KiB
    512 * 1024,  # 512KiB
    1 * 1024 * 1024,  # 1MiB
    2 * 1024 * 1024,  # 2MiB
    4 * 1024 * 1024,  # 4MiB
    8 * 1024 * 1024,  # 8MiB
    16 * 1024 * 1024,  # 16MiB
]


def run_dd_cmd(args):
    res = subprocess.run(
        args,
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        raise Exception("dd command failed")

    output = res.stderr

    idx = output.find("copied,")
    if idx == -1:
        raise Exception("failed to parse dd output")
    output = output[idx + len("copied,") :].strip()

    idx = output.find(" ")
    if idx == -1:
        raise Exception("failed to parse dd output")
    output = output[:idx].replace(",", ".")

    time = float(output)
    return time


def dd_write(block_size):
    if TRANSFER_SIZE % block_size != 0:
        raise Exception("invalid block_size")
    count = TRANSFER_SIZE // block_size

    time = run_dd_cmd(
        [
            "dd",
            "if=/dev/zero",
            "of=/dev/xdma0_h2c_0",
            f"bs={block_size}",
            f"count={count}",
        ]
    )
    speed = TRANSFER_SIZE / time
    return speed


def dd_read(block_size):
    if TRANSFER_SIZE % block_size != 0:
        raise Exception("invalid block_size")
    count = TRANSFER_SIZE // block_size

    time = run_dd_cmd(
        [
            "dd",
            "if=/dev/xdma0_c2h_0",
            "of=/dev/null",
            f"bs={block_size}",
            f"count={count}",
        ]
    )
    speed = TRANSFER_SIZE / time
    return speed


def bench_write():
    return [
        {"block_size": block_size, "speed": dd_write(block_size)}
        for block_size in BLOCK_SIZES
    ]


def run_benches(name):
    # Warmup
    print("Warming up...")
    dd_read(1 * 1024 * 1024)  # 1MiB
    dd_write(1 * 1024 * 1024)  # 1MiB

    # Read
    print("Running read benchmarks...")
    read = [
        {"block_size": block_size, "speed": dd_read(block_size)}
        for block_size in BLOCK_SIZES
    ]

    # Write
    print("Running write benchmarks...")
    write = [
        {"block_size": block_size, "speed": dd_write(block_size)}
        for block_size in BLOCK_SIZES
    ]

    # Save results
    print("Saving results...")
    with open(f"results/{name}.json", "w") as f:
        f.write(json.dumps({"read": read, "write": write}, indent=2))


def gen_readme():
    pass


if len(sys.argv) > 2:
    print("Invalid number of arguments")
elif len(sys.argv) == 2:
    run_benches(sys.argv[1])
    gen_readme()
else:
    gen_readme()
