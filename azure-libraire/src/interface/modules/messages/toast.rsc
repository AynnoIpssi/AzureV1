.az-toast { display: flex; flex-direction: row; gap: 12px; width: 340px; padding: 12px; border-radius: $rayon-grand; background-color: $surface-3; box-shadow: 0 8px 24px $ombre/45; }
.az-toast-bar { width: 4px; border-radius: 4px; background-color: $accent; flex-shrink: 0; }
.az-toast.succes .az-toast-bar { background-color: $succes; }
.az-toast.danger .az-toast-bar { background-color: $danger; }
.az-toast-body { display: flex; flex-direction: column; gap: 3px; }
.az-toast-title { color: $texte-fort; font-size: 14px; font-weight: 600; }
.az-toast-text { color: $texte-attenue; font-size: 13px; }
