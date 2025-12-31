#!/usr/bin/env python3

# Wrap around a command with perf and rusage to measure some metrics. Output the
# results either on stdout or to a file, according to the radar bench script
# specification.

import argparse
import json
import os
import resource
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path


@dataclass
class PerfMetric:
    event: str
    factor: float = 1
    unit: str | None = None


@dataclass
class RusageMetric:
    name: str
    factor: float = 1
    unit: str | None = None


PERF_UNITS = {
    "msec": 1e-3,
    "ns": 1e-9,
}

PERF_METRICS = {
    "task-clock": PerfMetric("task-clock", factor=1e-9, unit="s"),
    "wall-clock": PerfMetric("duration_time", factor=1e-9, unit="s"),
    "instructions": PerfMetric("instructions"),
    "cycles": PerfMetric("cycles"),
}

MAXRSS_FACTOR = 1000 if sys.platform.startswith("linux") else 1
RUSAGE_METRICS = {
    "maxrss": RusageMetric("ru_maxrss", factor=MAXRSS_FACTOR, unit="b"),
}


def measure_perf(cmd: list[str], events: list[str]) -> dict[str, tuple[float, str]]:
    with tempfile.NamedTemporaryFile() as tmp:
        cmd = [
            *["perf", "stat", "-j", "-o", tmp.name],
            *[arg for event in events for arg in ["-e", event]],
            *["--", *cmd],
        ]

        # Execute command
        env = os.environ.copy()
        env["LC_ALL"] = "C"  # or else perf may output syntactically invalid json
        result = subprocess.run(cmd, env=env)
        if result.returncode != 0:
            sys.exit(result.returncode)

        # Collect results
        perf = {}
        for line in tmp:
            data = json.loads(line)
            if "event" in data and "counter-value" in data:
                perf[data["event"]] = float(data["counter-value"]), data["unit"]

        return perf


@dataclass
class Result:
    category: str
    value: float
    unit: str | None

    def fmt(self, topic: str) -> str:
        metric = f"{topic}//{self.category}"
        if self.unit is None:
            return json.dumps({"metric": metric, "value": self.value})
        return json.dumps({"metric": metric, "value": self.value, "unit": self.unit})


def measure(cmd: list[str], metrics: list[str]) -> list[Result]:
    # Check args
    unknown_metrics = []
    for metric in metrics:
        if metric not in RUSAGE_METRICS and metric not in PERF_METRICS:
            unknown_metrics.append(metric)
    if unknown_metrics:
        raise Exception(f"unknown metrics: {', '.join(unknown_metrics)}")

    # Prepare perf events
    perf_metrics = [metric for metric in metrics if metric in PERF_METRICS]
    events: list[str] = []
    for metric in perf_metrics:
        if info := PERF_METRICS.get(metric):
            events.append(info.event)

    # Measure
    perf = {}
    elapsed = None
    perf_available = bool(perf_metrics) and shutil.which("perf") is not None
    if perf_metrics and perf_available:
        perf = measure_perf(cmd, events)
    else:
        if perf_metrics and not perf_available:
            missing = [
                metric
                for metric in perf_metrics
                if metric not in ("wall-clock", "task-clock")
            ]
            if missing:
                print(
                    "measure.py: perf not available, skipping metrics: "
                    + ", ".join(missing),
                    file=sys.stderr,
                )
        start = time.perf_counter()
        result = subprocess.run(cmd)
        if result.returncode != 0:
            sys.exit(result.returncode)
        elapsed = time.perf_counter() - start

    rusage = resource.getrusage(resource.RUSAGE_CHILDREN)

    # Extract results
    results = []
    for metric in metrics:
        if info := PERF_METRICS.get(metric):
            if perf:
                if info.event in perf:
                    value, unit = perf[info.event]
                else:
                    # Without the corresponding permissions,
                    # we only get access to the userspace versions of the counters.
                    value, unit = perf[f"{info.event}:u"]

                value *= PERF_UNITS.get(unit, info.factor)
                results.append(Result(metric, value, info.unit))
            elif metric == "wall-clock" and elapsed is not None:
                results.append(Result(metric, elapsed, "s"))
            elif metric == "task-clock":
                cpu = rusage.ru_utime + rusage.ru_stime
                results.append(Result(metric, cpu, "s"))

        if info := RUSAGE_METRICS.get(metric):
            value = getattr(rusage, info.name) * info.factor
            results.append(Result(metric, value, info.unit))

    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("-t", "--topic", action="append", default=[])
    parser.add_argument("-m", "--metric", action="append", default=[])
    parser.add_argument("-o", "--output", type=Path)
    parser.add_argument("cmd", nargs="*")
    args = parser.parse_args()

    topics: list[str] = args.topic
    metrics: list[str] = args.metric
    output: Path | None = args.output
    cmd: list[str] = args.cmd

    results = measure(cmd, metrics)

    if output:
        with open(output, "a+") as f:
            for result in results:
                for topic in topics:
                    f.write(f"{result.fmt(topic)}\n")
    else:
        for result in results:
            for topic in topics:
                print(f"radar::measurement={result.fmt(topic)}")
