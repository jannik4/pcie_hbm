import json
import os

import matplotlib.pyplot as plt
import numpy as np


def fmt_size(size):
    if size < 1024:
        return f"{size}B"
    elif size < 1024 * 1024:
        return f"{size / 1024}KiB"
    elif size < 1024 * 1024 * 1024:
        return f"{size / 1024 / 1024}MiB"
    else:
        return f"{size / 1024 / 1024 / 1024}GiB"


def fmt_speed(speed):
    if speed < 1024:
        return f"{speed:.2f}B/s"
    elif speed < 1024 * 1024:
        return f"{speed / 1024:.2f}KiB/s"
    elif speed < 1024 * 1024 * 1024:
        return f"{speed / 1024 / 1024:.2f}MiB/s"
    else:
        return f"{speed / 1024 / 1024 / 1024:.2f}GiB/s"


def gen_plots():
    # Read results
    results = []
    result_files = [f for f in os.listdir("results") if f.endswith(".json")]
    for file in result_files:
        with open(os.path.join("results", file), "r") as f:
            data = json.load(f)
            results.append(data)
    results.sort(key=lambda x: x["name"])

    if len(results) == 0:
        return

    # Generate plots
    opts = [
        {"key": "read", "title": "Read"},
        {"key": "write", "title": "Write"},
    ]
    for opt in opts:
        fig, ax = plt.subplots()
        for result in results:
            name = result["name"]
            data = result[opt["key"]]
            block_sizes = [fmt_size(v["block_size"]) for v in data]
            speeds = [v["speed"] for v in data]
            ax.plot(block_sizes, speeds, "o", label=name)
        ax.tick_params(axis="x", labelrotation=45)
        ax.yaxis.set_major_formatter(plt.FuncFormatter(lambda x, _: fmt_speed(x)))
        ax.set_xlabel("Block size")
        ax.set_ylabel("Speed")
        fig.suptitle(opt["title"])
        fig.legend()
        fig.savefig(f"results/{opt['key']}.svg", bbox_inches="tight")


def gen_readme():
    content = ""

    content += "> Note: This file is auto-generated. Do not edit manually. See [Run benchmarks](#run-benchmarks) for more information.\n\n"
    content += "# Benchmarks\n\n"

    content += "## Read\n\n"
    content += f"![Read](results/read.svg)\n\n"

    content += "## Write\n\n"
    content += f"![Write](results/write.svg)\n\n"

    content += "## Run benchmarks\n\n"
    content += """```sh
# Install dependencies
pip install -r requirements.txt

# Run benchmark for a specific design
sudo python3 bench.py <name_of_design>

# Generate plots and README
python3 gen.py
```
"""

    with open("README.md", "w") as f:
        f.write(content)


gen_plots()
gen_readme()
