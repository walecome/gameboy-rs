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

def build_emulator(release: bool):
    cmd = ['cargo', 'build']
    if release:
        cmd.append('--release')
    ret = subprocess.run(
        cmd,
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
    Crashed = auto()

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
    except KeyboardInterrupt:
        raise
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
        release: bool,
) -> TestResult:
    print(f"TEST: {rom.relative_to(SCRIPT_DIR)}")
    start_time = time.time()

    target = "release" if release else "debug"

    with subprocess.Popen(
        [
            f"target/{target}/gameboy-rs",
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

                if p.poll() is not None:
                    return TestResult(
                        rom=rom,
                        status=TestStatus.Crashed,
                        output="\n".join(streamed_output.read()),
                    )
                time.sleep(0.01)
        except:
            raise
        finally:
            p.terminate()

    return TestResult(
        rom=rom,
        status=TestStatus.Timeout,
        output="\n".join(streamed_output.read()),
    )

def get_test_roms(base_rom_dir: Path) -> List[Path]:
    return [
        base_rom_dir / 'cpu_instrs' / 'cpu_instrs.gb',
        *(base_rom_dir / 'cpu_instrs' / 'individual').glob("*.gb"),
        base_rom_dir / 'instr_timing' / 'instr_timing.gb',
        base_rom_dir / 'mem_timing' / 'mem_timing.gb',
        *(base_rom_dir / 'mem_timing' / 'individual').glob("*.gb"),
    ]

def emit_result(result: TestResult) -> bool:
    if result.status == TestStatus.Pass:
        print("OK")
        return True

    if result.status == TestStatus.Fail:
        print(f"Test failed")
    if result.status == TestStatus.Timeout:
        print(f"Test timed out")
    if result.status == TestStatus.Crashed:
        print(f"Test crashed")

    print("Test output: ")
    print("==========================================")
    print(result.output)
    print("==========================================")
    print()

    return False

def parse_args():
    parser = ArgumentParser()
    parser.add_argument(
        '--release',
        action='store_true',
        help='Run tests in release mode',
    )
    return parser.parse_args()

def main():
    test_rom_base_dir = SCRIPT_DIR / 'lib' / 'gb-test-roms'
    test_roms = get_test_roms(test_rom_base_dir)
    if not test_roms:
        print(f"No test roms found in dir: {test_rom_base_dir}")
        sys.exit(1)
    args = parse_args()
    build_emulator(release=args.release)

    all_passed = True
    for test_rom in test_roms:
        result = run_test(test_rom, timeout=50, release=args.release)
        all_passed &= emit_result(result)

    if all_passed:
        print("All tests passed")
    else:
        print("There were failing test")

    sys.exit(0 if all_passed else 1)

if __name__ == '__main__':
    main()
