/* Fenetres de demonstration : meme theme qu'Azure Docs (gris chauds, accent sable, pas de bleu). */
.fen { display: flex; flex-direction: column; gap: 12px; height: 100%; padding: 26px 30px; background-color: #121212; color: #e7e5e1; font-size: 15px; }
.fen-surtitre { color: #c9a878; font-size: 12px; font-weight: 700; letter-spacing: 1px; }
.fen-titre { color: #f0eeea; font-size: 26px; font-weight: 700; }
.fen-texte { color: #b8b5ae; font-size: 14px; line-height: 1.55; }
.fen-pied { margin-top: auto; color: #6f6c66; font-size: 12px; }
.etiquette { margin-top: 6px; color: #8a877f; font-size: 12px; font-weight: 700; letter-spacing: 1px; }
.ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; }

.saisie { flex-grow: 1; height: 38px; padding: 0 12px; background-color: #1e1e1e; border: 1px solid #2c2c2c; border-radius: 8px; color: #e7e5e1; font-size: 14px; }
.saisie:focus { border: 1px solid #c9a878; }
.principal { height: 38px; padding: 0 18px; background-color: #c9a878; border-radius: 8px; color: #1a1712; font-size: 14px; font-weight: 700; cursor: pointer; transition: background-color 0.15s ease; }
.principal:hover { background-color: #d8b98a; }
.secondaire { width: 64px; height: 36px; background-color: #1e1e1e; border: 1px solid #2e2c29; border-radius: 8px; color: #e7e5e1; font-size: 15px; font-weight: 700; cursor: pointer; transition: background-color 0.15s ease; }
.secondaire:hover { background-color: #26241f; }

.pastille { height: 34px; padding: 0 14px; border-radius: 17px; color: #1a1712; font-size: 13px; font-weight: 700; cursor: pointer; }
.sable { background-color: #c9a878; }
.sauge { background-color: #8fa585; }
.terracotta { background-color: #c07a5a; }
.ardoise { background-color: #8a857d; }

.panneau { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; height: 150px; border-radius: 12px; transition: background-color 0.3s ease; }
.panneau.vide { background-color: #1a1917; border: 1px dashed #3a3530; }
.panneau-message { color: #1a1712; font-size: 20px; font-weight: 700; }
.panneau-compteur { color: #1a1712; font-size: 40px; font-weight: 700; }
.panneau.vide .panneau-message { color: #8a877f; font-size: 15px; font-weight: 400; }
.panneau.vide .panneau-compteur { color: #4d4a45; }
.journal { display: flex; flex-direction: row; gap: 10px; padding: 8px 12px; background-color: #181818; border: 1px solid #242424; border-radius: 8px; }
.journal-l { width: 130px; color: #8a877f; font-size: 13px; }
.journal-v { color: #e0cfb3; font-size: 13px; flex-grow: 1; }

.gros { color: #e0cfb3; font-size: 56px; font-weight: 700; }
.centre { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 10px 0; }
.taille { color: #f0eeea; font-size: 44px; font-weight: 700; }
