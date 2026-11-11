# Vizia (Optimized)

Uses [Vizia PR #781](https://github.com/vizia/vizia/pull/781), pinned to `edb68af29d4982ae46e347d4a7252fa26ecaa50c`, with the same workloads as the regular Vizia app.

Open the dashboard:

```sh
just app vizia-optimized dashboard
```

Run automated dashboard captures:

```sh
python3 apps/bench.py dashboard vizia-optimized --repetitions 2
```
