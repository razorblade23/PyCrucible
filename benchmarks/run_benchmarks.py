#!/usr/bin/env python3
import os
import time
import json
import subprocess
import shutil
import io
import zipfile
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
    import hashlib
    result = {"project": name}
    # ensure per-project output directory and unique absolute output file
    project_dir = Path(project_dir)
    dist_dir = project_dir / "dist"
    dist_dir.mkdir(exist_ok=True)
    # use a timestamped filename to avoid accidental reuse/collision
    output_file = dist_dir / f"{name}-{int(time.time()*1000)}"
    # remove any previous file for a clean measurement (none expected with timestamp)
    if output_file.exists():
        output_file.unlink()
    binary_path = output_file

    # Embed
    pycrucible_path = ROOT.parent / "target" / "release" / "pycrucible"
    embed_cmd = [str(pycrucible_path), "-e", str(project_dir), "-o", str(output_file), "--debug"]
    t_embed, code, out, err = timed(embed_cmd, cwd=project_dir)
    result["embed_time"] = round(t_embed, 2)
    result["embed_success"] = (code == 0)

    # always print stdout/stderr for diagnosis
    print(f"[embed stdout]\n{out}", flush=True)
    print(f"[embed stderr]\n{err}", flush=True)
    print(f"[embed exit code] {code}", flush=True)

    if not output_file.exists():
        # fallback: search project_dir/dist for newest file
        candidates = list((project_dir / "dist").glob("*"))
        if candidates:
            candidates.sort(key=lambda p: p.stat().st_mtime, reverse=True)
            binary_path = candidates[0]
            print(f"[debug] fallback picked: {binary_path}", flush=True)
        else:
            binary_path = output_file

    if binary_path.exists():
        # detailed diagnostics
        st = binary_path.stat()
        size_bytes = st.st_size
        mtime = st.st_mtime
        # compute sha256 (only first/second run need this diagnostic)
        h = hashlib.sha256()
        with open(binary_path, "rb") as fh:
            for chunk in iter(lambda: fh.read(8192), b""):
                h.update(chunk)
        sha = h.hexdigest()

        print(f"[debug] binary: {binary_path} size={size_bytes} mtime={mtime} sha256={sha}", flush=True)
        # record exact bytes and a more-precise MiB value so small differences are visible
        result["binary_size_bytes"] = size_bytes
        # use MiB (1024*1024) and keep 3 decimal places
        result["binary_size_mb"] = round(size_bytes / 1024.0 / 1024.0, 3)
        result["binary_sha256"] = sha

        # Try to locate embedded zip inside the binary (look for PK signature)
        embedded_files = []
        try:
            with open(binary_path, "rb") as bf:
                data = bf.read()
            pk = data.find(b"PK\x03\x04")
            if pk != -1:
                try:
                    z = zipfile.ZipFile(io.BytesIO(data[pk:]))
                    embedded_files = z.namelist()
                except zipfile.BadZipFile:
                    print(f"[debug] found PK header at {pk} but failed to open zip", flush=True)
            else:
                print(f"[debug] no embedded PK zip header found in {binary_path}", flush=True)
        except Exception as e:
            print(f"[debug] error while scanning for embedded zip: {e}", flush=True)

        result["embedded_files"] = embedded_files
        # write per-project embedded file list for easier artifact inspection
        try:
            if embedded_files:
                (RESULTS_DIR / f"{name}_embedded_files.txt").write_text("\n".join(embedded_files))
        except Exception as e:
            print(f"[debug] failed to write embedded files list: {e}", flush=True)

        # Run first time (cold start)
        t1, code1, out1, err1 = timed([str(binary_path)])
        print(f"[run1 stdout]\n{out1}", flush=True)
        print(f"[run1 stderr]\n{err1}", flush=True)
        result["run_first_time"] = round(t1, 2)
        result["run_first_success"] = (code1 == 0)

        # Run second time (warm cache)
        t2, code2, out2, err2 = timed([str(binary_path)])
        print(f"[run2 stdout]\n{out2}", flush=True)
        print(f"[run2 stderr]\n{err2}", flush=True)
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
        # "many_files"
    ]
    results = []
    for proj in projects:
        path = ROOT / "projects" / proj
        print(f"\nBenchmarking {proj} ...", flush=True)
        results.append(measure_project(proj, path))
        print(f"Done benchmarking {proj}.\n", flush=True)

    if not RESULTS_DIR.exists():
        RESULTS_DIR.mkdir()

    with open(RESULTS_DIR / "results.json", "w") as f:
        json.dump(results, f, indent=2)

    # Create markdown summary
    md = ["# PyCrucible Benchmark Results\n"]
    md.append("| Project | Embed Time (s) | Size (MiB / bytes) | First Run (s) | Second Run (s) | Success |")
    md.append("|----------|----------------|-----------|---------------|----------------|----------|")
    for r in results:
        size_mb = r.get('binary_size_mb', '?')
        size_bytes = r.get('binary_size_bytes', '?')
        md.append(f"| {r['project']} | {r.get('embed_time','?')} | {size_mb} MiB / {size_bytes} B | {r.get('run_first_time','?')} | {r.get('run_second_time','?')} | {'OK' if r.get('embed_success') else 'Not OK'} |")
    (RESULTS_DIR / "results.md").write_text("\n".join(md))
    print("\n".join(md), flush=True)

if __name__ == "__main__":
    main()
