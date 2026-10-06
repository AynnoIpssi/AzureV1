.az-status { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.az-status-dot { width: 8px; height: 8px; border-radius: $rayon-rond; background-color: $accent; flex-shrink: 0; }
.az-status-text { color: $texte-doux; font-size: 13px; }
.az-status.succes .az-status-dot { background-color: $succes; }
.az-status.attention .az-status-dot { background-color: $attention; }
.az-status.danger .az-status-dot { background-color: $danger; }
.az-status.neutre .az-status-dot { background-color: $texte-eteint; }
