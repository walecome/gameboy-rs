#!/usr/bin/env python3

import sys
import subprocess
import time
import fcntl
import os

from argparse import ArgumentParser
from pathlib import Path
from typing import IO, List, Optional
from enum import Enum, auto
from dataclasses import dataclass
SCRIPT_DIR = Path(__file__).parent

def build_emulator():
    ret = subprocess.run(
        ['cargo', 'build'],
        capture_output=True,
        cwd=SCRIPT_DIR,
    )
    if ret.returncode != 0:
        print(ret.stderr)
        print(ret.stdout)
        ret.check_returncode()

class TestStatus(Enum):
    Pass = auto()
    Fail = auto()
    Timeout = auto()

@dataclass
class TestResult:
    rom: Path
    status: TestStatus
    output: str

def non_block_read(output):
    try:
        fd = output.fileno()
        fl = fcntl.fcntl(fd, fcntl.F_GETFL)
        fcntl.fcntl(fd, fcntl.F_SETFL, fl | os.O_NONBLOCK)
        return output.read()
    except:
        return None

class StreamedOutput:
    def __init__(self, stream: IO[str]):
        self._stream = stream
        self._content = ""

    def read(self) -> List[str]:
        new_content = non_block_read(self._stream)
        if new_content is not None:
            self._content += new_content
        return self._content.splitlines()

def try_extract_result(output: StreamedOutput) -> Optional[TestStatus]:
    for line in reversed(output.read()):
        if "Passed" in line:
            return TestStatus.Pass
        if "Failed" in line:
            return TestStatus.Fail

    return None

def run_test(
        rom: Path,
        timeout: int,
) -> TestResult:
    print(f"Running test for ROM: {rom}")
    start_time = time.time()

    with subprocess.Popen(
        [
            'target/debug/gameboy-rs',
            '--rom', rom,
            '--headless',
            '--trace-mode', 'serial',
        ],
        cwd=SCRIPT_DIR,
        universal_newlines=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ) as p:
        try:
            if p.stdout is None:
                raise RuntimeError("stdout is None")
            streamed_output = StreamedOutput(p.stdout)

            while time.time() - start_time < timeout:
                maybe_status = try_extract_result(streamed_output)
                if maybe_status is not None:
                    return TestResult(
                        rom=rom,
                        status=maybe_status,
                        output="\n".join(streamed_output.read()),
                    )
        except:
            raise
        finally:
            p.terminate()

    return TestResult(
        rom=rom,
        status=TestStatus.Timeout,
        output="\n".join(streamed_output.read()),
    )

def parse_args():
    parser = ArgumentParser()

    parser.add_argument('test_rom_dir', type=Path)

    return parser.parse_args()

def get_test_roms(rom_dir: Path) -> List[Path]:
    return list(rom_dir.glob("*.gb"))

def emit_results(results: List[TestResult]):
    any_failed = False
    for result in results:
        if result.status == TestStatus.Pass:
            continue
        any_failed = True
        if result.status == TestStatus.Fail:
            print(f"Test failed: {result.rom}")
        if result.status == TestStatus.Timeout:
            print(f"Test timed out: {result.rom}")

        print("Test output: ")
        print("==========================================")
        print(result.output)
        print("==========================================")
        print()

    if any_failed:
        print("There were failing tests")
    else:
        print("All tests passed")

def main():
    args = parse_args()
    test_roms = get_test_roms(args.test_rom_dir)
    if not test_roms:
        print(f"No test roms found in dir: {args.test_rom_dir}")
        sys.exit(1)
    build_emulator()
    results = [run_test(test_rom, 10) for test_rom in test_roms]
    emit_results(results)

if __name__ == '__main__':
    main()
