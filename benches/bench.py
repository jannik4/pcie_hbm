import subprocess
import json


def dd_write(block_size):
    amount = 1024 * 1024 * 1024  # 1GiB
    if amount % block_size != 0:
        raise Exception("invalid block_size")
    count = amount // block_size

    res = subprocess.run(
        [
            "dd",
            "if=/dev/xdma0_c2h_0",
            "of=/dev/null",
            f"bs={block_size}",
            f"count={count}",
        ],
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        raise Exception("dd failed")

    output = res.stderr
    output = output[output.find("copied,") + 7 :].strip()
    output = output[: output.find(" ")].replace(",", ".")
    time = float(output)

    speed = amount / time
    return speed


def bench_write():
    block_sizes = [
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
    return [
        {"block_size": block_size, "speed": dd_write(block_size)}
        for block_size in block_sizes
    ]


print(json.dumps(bench_write(), indent=2))
