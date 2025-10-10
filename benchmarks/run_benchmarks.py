#!/usr/bin/env python3
import os
import time
import json
import subprocess
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent
RESULTS_DIR = ROOT / "results"
RESULTS_DIR.mkdir(exist_ok=True, parents=True)

def timed(cmd, cwd=None):
    """Run a command and return elapsed time + stdout/stderr."""
    start = time.perf_counter()
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    end = time.perf_counter()
    return end - start, proc.returncode, proc.stdout, proc.stderr

def measure_project(name, project_dir):
    """Run full PyCrucible cycle for one project."""
    result = {"project": name}
    binary_path = Path(project_dir) / f"{name}"
    if binary_path.exists():
        binary_path.unlink()

    if name == "many_files":
        return  # Skip for now
        # Generate many files
        gen_script = Path(project_dir) / "generate.py"
        if gen_script.exists():
            print("Generating many files...")
            t_gen, code_gen, out_gen, err_gen = timed(["python3", str(gen_script)], cwd=project_dir)
            result["generate_time"] = round(t_gen, 2)
            if code_gen != 0:
                result["error"] = "generation failed"
                print(out_gen)
                print(err_gen)

    # Embed
    pycrucible_path = ROOT.parent / "target" / "release" / "pycrucible"
    embed_cmd = [pycrucible_path, "-e", ".", "-o", f"projects/{project_dir}/{name}", "--debug"]
    t_embed, code, out, err = timed(embed_cmd, cwd=project_dir)
    result["embed_time"] = round(t_embed, 2)
    result["embed_success"] = (code == 0)
    print(out)
    if code != 0:
        print(err)

    if not binary_path.exists():
        # Fallback: try finding binary
        for f in Path(project_dir, "dist").glob("*"):
            binary_path = f
            break

    if binary_path.exists():
        result["binary_size_mb"] = round(binary_path.stat().st_size / 1_000_000, 2)

        # Run first time (cold start)
        t1, code1, out1, err1 = timed([str(binary_path)])
        result["run_first_time"] = round(t1, 2)
        result["run_first_success"] = (code1 == 0)

        # Run second time (warm cache)
        t2, code2, out2, err2 = timed([str(binary_path)])
        result["run_second_time"] = round(t2, 2)
        result["run_second_success"] = (code2 == 0)
    else:
        result["error"] = "binary not found"

    return result

def main():
    projects = [
        "cowsay_app",
        "flask_app",
        "fastapi_app",
        "pygame_app",
        "heavy_deps",
        "many_files"
    ]
    results = []
    for proj in projects:
        path = ROOT / "projects" / proj
        print(f"\n🏗️  Benchmarking {proj} ...")
        results.append(measure_project(proj, path))

    if not RESULTS_DIR.exists():
        RESULTS_DIR.mkdir()

    with open(RESULTS_DIR / "results.json", "w") as f:
        json.dump(results, f, indent=2)

    # Create markdown summary
    md = ["# PyCrucible Benchmark Results\n"]
    md.append("| Project | Embed Time (s) | Size (MB) | First Run (s) | Second Run (s) | Success |")
    md.append("|----------|----------------|-----------|---------------|----------------|----------|")
    for r in results:
        md.append(f"| {r['project']} | {r.get('embed_time','?')} | "
                  f"{r.get('binary_size_mb','?')} | {r.get('run_first_time','?')} | "
                  f"{r.get('run_second_time','?')} | {'✅' if r.get('embed_success') else '❌'} |")
    (RESULTS_DIR / "results.md").write_text("\n".join(md))
    print("\n".join(md))

if __name__ == "__main__":
    main()
