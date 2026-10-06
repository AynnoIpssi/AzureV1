.az-modal-backdrop { position: fixed; top: 0; right: 0; bottom: 0; left: 0; z-index: 100; display: flex; flex-direction: column; align-items: center; justify-content: center; background-color: $voile/62; }
.az-modal { display: flex; flex-direction: column; gap: 12px; width: 460px; max-width: 90%; padding: 18px 20px; background-color: $surface-2; border: 1px solid $trait/8; border-radius: $rayon-grand; box-shadow: 0 18px 50px $ombre/55; }
.az-modal-head { display: flex; flex-direction: row; align-items: center; gap: 10px; }
.az-modal-title { flex-grow: 1; color: $texte-fort; font-size: 17px; font-weight: 700; }
.az-modal-close { width: 30px; height: 30px; padding: 0; border-radius: $rayon; background-color: $trait/6; color: $texte-attenue; font-size: 13px; }
.az-modal-close:hover { background-color: $trait/12; }
.az-modal-body { display: flex; flex-direction: column; gap: 10px; color: $texte-doux; font-size: 14px; }
