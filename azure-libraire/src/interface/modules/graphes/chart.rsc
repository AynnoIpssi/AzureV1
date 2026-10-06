.az-chart { display: flex; flex-direction: column; gap: 10px; padding: 14px 16px; background-color: $surface; border: 1px solid $trait/6; border-radius: $rayon-grand; min-width: 0; }
.az-chart.nu { padding: 0; background-color: transparent; border: 0px solid transparent; }
.az-chart-head { display: flex; flex-direction: row; align-items: flex-start; gap: 12px; }
.az-chart-main { display: flex; flex-direction: column; gap: 2px; flex-grow: 1; min-width: 0; }
.az-chart-title { color: $texte-doux; font-size: 13px; font-weight: 600; }
.az-chart-detail { color: $texte-faible; font-size: 12px; }
.az-chart-value { color: $texte-fort; font-size: 18px; font-weight: 700; flex-shrink: 0; }
.az-chart-help { color: $texte-faible; font-size: 12px; }
.az-chart-plot { width: 100%; height: 160px; }
.az-chart.petit .az-chart-plot { height: 96px; }
.az-chart.grand .az-chart-plot { height: 240px; }
.az-plot-jauge { height: 130px; }
.az-plot-radar { height: 220px; }
