"""Frame-budget breakdown for the temporal_core latency audit.

Measured on a Neoverse-N1 VM, Rust 1.99.1, release + thin LTO, synthetic SMS ROM
(`jr $`), 59,736 T-states/frame, 49,780 scheduler dispatches/frame.
"""
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt

# Common categories, plotted top-to-bottom in this order on both panels.
CATEGORIES = [
    "Scheduler dispatch",
    "PSG",
    "VDP advance",
    "CPU Z80 + M1 + barramento\n+ render de scanline",
]
CURRENT = [6.80, 1.22, 0.50, 7.30]
AFTER = [0.25, 0.15, 0.15, 7.30]
LABEL_CURRENT = ["6,80 ms\n49.780 eventos", "1,22 ms\nmedido", "0,50 ms\nestimado", "7,30 ms"]
LABEL_AFTER = ["0,25 ms\n~10.000 eventos", "0,15 ms", "0,15 ms", "7,30 ms"]
COLORS = ["#c0392b", "#e67e22", "#f1c40f", "#7f8c8d"]

fig, axes = plt.subplots(1, 2, figsize=(13.5, 5.8), sharey=True)
panels = (
    (axes[0], CURRENT, LABEL_CURRENT, "Hoje — 15,9 ms/frame (95% do teto de 16,67 ms)"),
    (axes[1], AFTER, LABEL_AFTER, "Com P0–P3 — ~7,9 ms/frame (47%)"),
)

positions = list(range(len(CATEGORIES)))
for ax, values, labels, title in panels:
    bars = ax.barh(positions, values, color=COLORS, height=0.58)
    for bar, value, label in zip(bars, values, labels):
        ax.text(
            value + 0.25,
            bar.get_y() + bar.get_height() / 2,
            label,
            va="center",
            fontsize=9,
            linespacing=1.15,
        )
    ax.axvline(16.667, color="#2c3e50", linestyle="--", linewidth=1.3)
    ax.set_xlim(0, 19.5)
    ax.set_title(title, fontsize=10.5, pad=10)
    ax.set_xlabel("ms por frame a 60 FPS")
    ax.set_yticks(positions)
    ax.set_yticklabels(CATEGORIES)
    ax.grid(axis="x", alpha=0.25)
    ax.set_axisbelow(True)
    ax.invert_yaxis()

axes[1].text(
    16.2,
    3.42,
    "teto 16,67 ms",
    ha="right",
    va="center",
    fontsize=8.5,
    color="#2c3e50",
    style="italic",
)

fig.suptitle(
    "SUN — onde o frame time é gasto (SMS NTSC, 59.736 T-states/frame)",
    fontsize=13,
    fontweight="bold",
)
fig.tight_layout()
fig.savefig("/home/ubuntu/SUN/docs/audits/figures/temporal-latency-budget.png", dpi=150)
print("ok")
