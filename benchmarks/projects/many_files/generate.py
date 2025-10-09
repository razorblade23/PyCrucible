import os
os.makedirs("modules", exist_ok=True)
for i in range(1000):
    with open(f"modules/mod_{i}.py", "w") as f:
        f.write(f"def f(): return {i}\n")

with open("main.py", "w") as f:
    f.write("import modules.mod_999; print('Loaded 1000 modules!')\n")
