/* La boite « Ouvrir » d'Azure (voir selecteur.rsh) : le meme sombre sobre
   que les modales (`az-modal`), l'accent sable. */
.sel-fond { position: fixed; top: 0; right: 0; bottom: 0; left: 0; z-index: 1000; display: flex; flex-direction: column; align-items: center; justify-content: center; background-color: rgba(8, 8, 7, 0.62); }
.sel-boite { display: flex; flex-direction: column; gap: 10px; width: 780px; max-width: 94%; height: 540px; max-height: 92%; padding: 16px 18px; background-color: #161514; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 14px; box-shadow: 0 18px 50px rgba(0, 0, 0, 0.55); }

.sel-tete { display: flex; flex-direction: row; align-items: center; gap: 10px; flex-shrink: 0; }
.sel-titre { flex-grow: 1; color: #f3f1ed; font-size: 16px; font-weight: 700; }
.sel-x { width: 28px; height: 28px; padding: 0; border-radius: 8px; background-color: rgba(255, 255, 255, 0.06); color: #b8b5ae; font-size: 13px; cursor: pointer; }
.sel-x:hover { background-color: rgba(255, 255, 255, 0.12); }

.sel-barre { display: flex; flex-direction: row; align-items: center; gap: 8px; flex-shrink: 0; }
.sel-outil { height: 28px; padding: 0 10px; flex-shrink: 0; background-color: #1f1e1b; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 7px; color: #c9c6bf; font-size: 12px; cursor: pointer; }
.sel-outil:hover { background-color: #282622; }
.sel-outil.on { border: 1px solid #c9a878; color: #e0cfb3; }
.sel-fil { display: flex; flex-direction: row; align-items: center; flex-wrap: wrap; gap: 2px; flex-grow: 1; min-width: 0; min-height: 28px; padding: 0 6px; background-color: #1b1a18; border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 7px; }
.sel-fil-b { height: 24px; padding: 0 5px; background-color: transparent; border-radius: 5px; color: #aeaba3; font-size: 12.5px; cursor: pointer; }
.sel-fil-b:hover { background-color: rgba(255, 255, 255, 0.07); color: #f0eeea; }
.sel-fil-sep { color: #5f5c56; font-size: 12.5px; }
.sel-fil-ici { padding: 0 5px; color: #f0eeea; font-size: 12.5px; font-weight: 600; }

.sel-corps { display: flex; flex-direction: row; gap: 10px; flex-grow: 1; min-height: 0; }
.sel-lieux { display: flex; flex-direction: column; gap: 2px; width: 200px; flex-shrink: 0; padding: 8px 6px; background-color: #1b1a18; border-radius: 9px; overflow-y: auto; }
.sel-petit { padding: 0 6px 4px 6px; color: #7d7a73; font-size: 10.5px; font-weight: 700; letter-spacing: 0.6px; }
.sel-lieu { height: 28px; padding: 0 8px; flex-shrink: 0; background-color: transparent; border-radius: 6px; color: #c9c6bf; font-size: 12.5px; text-align: left; cursor: pointer; }
.sel-lieu:hover { background-color: rgba(255, 255, 255, 0.05); }
.sel-lieu.ici { background-color: rgba(201, 168, 120, 0.13); color: #f0eeea; font-weight: 600; }

.sel-liste { display: flex; flex-direction: column; gap: 1px; flex-grow: 1; min-width: 0; min-height: 0; padding: 6px; background-color: #1b1a18; border-radius: 9px; overflow-y: auto; }
.sel-ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 28px; padding: 0 0 0 8px; flex-shrink: 0; border-radius: 6px; }
.sel-ligne:hover { background-color: rgba(255, 255, 255, 0.04); }
.sel-ic { width: 11px; height: 9px; flex-shrink: 0; border-radius: 2px; }
.sel-ic.dossier { background-color: #c9a878; }
.sel-ic.fichier { width: 9px; height: 11px; margin: 0 1px; border: 1.5px solid #76736c; }
.sel-nom { flex-grow: 1; min-width: 0; height: 28px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #e7e5e1; font-size: 13px; text-align: left; cursor: pointer; }
.sel-nom.choisi { background-color: rgba(201, 168, 120, 0.16); color: #f6f1e7; font-weight: 600; }
.sel-nom.inactif { height: auto; color: #6f6c66; cursor: default; }
.sel-erreur { padding: 10px 12px; color: #fca5a5; font-size: 13px; line-height: 1.45; }
.sel-aide { padding: 8px 8px; color: #7d7a73; font-size: 12.5px; }

.sel-pied { display: flex; flex-direction: row; align-items: center; gap: 8px; flex-shrink: 0; }
.sel-choix { flex-grow: 1; min-width: 0; color: #aeaba3; font-family: monospace; font-size: 12px; }
.sel-bouton { height: 32px; padding: 0 16px; flex-shrink: 0; background-color: #1f1e1b; border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; color: #e7e5e1; font-size: 13px; font-weight: 500; cursor: pointer; }
.sel-bouton:hover { background-color: #282622; }
.sel-bouton.fort { background-color: #c9a878; border: 1px solid #c9a878; color: #1a1712; font-weight: 600; }
.sel-bouton.fort:hover { background-color: #d8b98a; }
.sel-bouton.eteint { height: auto; padding: 8px 16px; color: #6f6c66; cursor: default; }
