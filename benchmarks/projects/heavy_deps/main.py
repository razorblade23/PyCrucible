import numpy as np, pandas as pd, matplotlib.pyplot as plt
df = pd.DataFrame({"a": np.arange(10), "b": np.arange(10)**2})
df.to_csv("out.csv", index=False)
print(df.head())
plt.plot(df.a, df.b)
plt.savefig("plot.png")
