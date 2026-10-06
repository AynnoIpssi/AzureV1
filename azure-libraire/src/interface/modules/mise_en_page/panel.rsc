.az-panel { display: flex; flex-direction: column; background-color: $surface; border: 1px solid $trait/6; border-radius: $rayon-grand; }
.az-panel-head { display: flex; flex-direction: row; align-items: center; padding: 10px 14px; background-color: $surface-2; border-bottom: 1px solid $trait/6; flex-shrink: 0; }
.az-panel-title { color: $texte-discret; font-size: 12px; font-weight: 700; }
.az-panel-body { display: flex; flex-direction: column; gap: 10px; padding: 14px; flex-grow: 1; min-height: 0; }
.az-panel.plat .az-panel-body { padding: 0; gap: 0; }
