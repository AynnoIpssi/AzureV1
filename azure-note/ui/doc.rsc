/* La fenetre Doc d'Azure Note : le rendu d'Azure Docs, en plus compact. */
.app { display: flex; flex-direction: column; height: 100%; background-color: #121212; color: #e7e5e1; font-size: 14px; scrollbar-color: #3b3935; }

.haut { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 54px; padding: 0 16px; flex-shrink: 0; background-color: #161616; border-bottom: 1px solid #272727; }
.source { color: #c9a878; font-size: 11px; font-weight: 700; letter-spacing: 1px; margin-right: 6px; }
.champ { flex-grow: 1; height: 32px; padding: 0 12px; background-color: #1c1b19; border: 1px solid #2e2c29; border-radius: 8px; color: #e7e5e1; font-size: 14px; }
.champ:focus { border: 1px solid #c9a878; }
.chercher { height: 32px; padding: 0 16px; background-color: #c9a878; border-radius: 8px; color: #1a1712; font-size: 13px; font-weight: 700; cursor: pointer; }
.chercher:hover { background-color: #d8b98a; }
.chercher:active { background-color: #b39463; }

.page { display: flex; flex-direction: column; gap: 12px; flex-grow: 1; min-height: 0; padding: 20px 24px 28px 24px; overflow-y: auto; }
.lead { color: #8a877f; font-size: 13px; }

.res { display: flex; flex-direction: column; gap: 4px; padding: 12px 14px; background-color: #171717; border: 1px solid #242424; border-radius: 10px; flex-shrink: 0; }
.res:hover { border: 1px solid #3a352d; }
.res-tete { display: flex; flex-direction: row; align-items: center; gap: 10px; }
.res-genre { padding: 1px 8px; background-color: #1f1d1a; border-radius: 4px; color: #c9a878; font-size: 11px; font-weight: 600; }
.res-genre.g-exemple { background-color: #1a1c18; color: #b9cbb0; }
.res-titre { height: 22px; padding: 0; background-color: transparent; color: #f0eeea; font-size: 15px; font-weight: 600; text-align: left; cursor: pointer; }
.res-titre:hover { color: #e0cfb3; }
.res-titre:active { background-color: transparent; color: #c9a878; }
.res-lieu { color: #6f6c66; font-size: 12px; }
.res-extrait { color: #a9a69f; font-size: 13px; line-height: 1.5; }

.fil { display: flex; flex-direction: row; align-items: center; gap: 10px; }
.retour { height: 28px; padding: 0 12px; background-color: #1e1e1e; border: 1px solid #2e2c29; border-radius: 6px; color: #b8b5ae; font-size: 12px; font-weight: 600; cursor: pointer; }
.retour:hover { background-color: #26241f; color: #e0cfb3; }
.fil-section { color: #6f6c66; font-size: 13px; }
.ouvrir { margin-left: auto; height: 28px; padding: 0 12px; background-color: rgba(201, 168, 120, 0.12); border: 1px solid rgba(201, 168, 120, 0.35); border-radius: 6px; color: #e0cfb3; font-size: 12px; font-weight: 600; cursor: pointer; }
.ouvrir:hover { background-color: rgba(201, 168, 120, 0.2); }
.vise-titre { color: #c9a878; font-size: 11px; font-weight: 700; letter-spacing: 1px; margin-top: 4px; }
.code.vise { border: 1px solid #5a4a33; }

.titre { color: #f0eeea; font-size: 26px; font-weight: 700; margin-top: 4px; }
.resume { color: #9d9a93; font-size: 15px; line-height: 1.5; }
.filet { height: 1px; margin: 6px 0 2px 0; flex-shrink: 0; background-color: #262626; }

.erreur { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; padding: 16px 18px; background-color: #1c1816; border-left: 3px solid #c98a6b; border-radius: 4px; }
.erreur-titre { color: #e0ad94; font-size: 12px; font-weight: 700; letter-spacing: 1px; }
.erreur-texte { color: #c9c6bf; font-size: 14px; line-height: 1.5; }
.btn { height: 30px; padding: 0 14px; background-color: #1e1e1e; border: 1px solid #2e2c29; border-radius: 6px; color: #b8b5ae; font-size: 13px; font-weight: 600; cursor: pointer; }
.btn:hover { background-color: #26241f; color: #e7e5e1; }

/* ---- blocs de page : repris d'azure-docs/ui/docs.rsc ---- */
.h2 { color: #f0eeea; font-size: 21px; font-weight: 700; margin-top: 22px; }
.h3 { color: #e7e5e1; font-size: 17px; font-weight: 600; margin-top: 12px; }
.p { color: #c9c6bf; font-size: 15px; line-height: 1.6; }

.liste { display: flex; flex-direction: column; gap: 6px; padding-left: 4px; }
.li { display: flex; flex-direction: row; gap: 10px; }
.puce { color: #c9a878; font-size: 15px; flex-shrink: 0; }
.li-texte { color: #c9c6bf; font-size: 15px; line-height: 1.55; flex-grow: 1; min-width: 0; }

.note { display: flex; flex-direction: column; gap: 4px; padding: 12px 16px; background-color: #1a1917; border-left: 3px solid #c9a878; border-radius: 4px; }
.note.astuce { background-color: #181a17; border-left: 3px solid #9bb08f; }
.note.attention { background-color: #1c1816; border-left: 3px solid #c98a6b; }
.note-titre { color: #e0cfb3; font-size: 12px; font-weight: 700; letter-spacing: 1px; }
.note.astuce .note-titre { color: #b9cbb0; }
.note.attention .note-titre { color: #e0ad94; }
.note-texte { color: #c9c6bf; font-size: 14px; line-height: 1.55; }

.tableau { display: flex; flex-direction: column; border: 1px solid #272727; border-radius: 8px; }
.tr { display: flex; flex-direction: row; border-top: 1px solid #242424; }
.tr.entete { background-color: #1a1a1a; border-top: 0px solid #242424; }
.th { flex-grow: 1; flex-basis: 0; min-width: 0; padding: 9px 12px; color: #e7e5e1; font-size: 13px; font-weight: 600; }
.td { flex-grow: 1; flex-basis: 0; min-width: 0; padding: 9px 12px; color: #b8b5ae; font-size: 13px; line-height: 1.45; }

.code { display: flex; flex-direction: column; background-color: #0e0e0e; border: 1px solid #242424; border-radius: 8px; flex-shrink: 0; }
.code-tete { display: flex; flex-direction: row; align-items: center; gap: 10px; padding: 8px 14px; background-color: #151515; border-bottom: 1px solid #222222; }
.code-lang { padding: 1px 8px; background-color: #1f1d1a; border-radius: 4px; color: #c9a878; font-family: monospace; font-size: 12px; }
.code-titre { color: #9d9a93; font-size: 13px; }
.code-id { margin-left: auto; color: #6f6c66; font-family: monospace; font-size: 12px; }
.code-copier { width: 78px; height: 26px; background-color: #1e1e1e; border: 1px solid #2e2c29; border-radius: 6px; color: #b8b5ae; font-size: 12px; font-weight: 600; cursor: pointer; transition: background-color 0.15s ease; }
.code-copier:hover { background-color: #26241f; color: #e0cfb3; }
/* le code garde sa largeur (lignes longues) : son conteneur defile a l'horizontale */
.code-corps { display: flex; flex-direction: row; overflow-x: auto; }
.code-lignes { display: flex; flex-direction: column; flex-shrink: 0; padding: 14px 16px; }
.code-ligne { display: flex; flex-direction: row; }
.tk { flex-shrink: 0; color: #dcd8cf; font-family: monospace; font-size: 13px; line-height: 1.5; white-space: pre; }
/* coloration : tons chauds et sourds, aucun bleu */
.mot { color: #cf8e6d; }
.balise { color: #d6b37c; }
.classe { color: #b9a0b4; }
.attr { color: #c2b59b; }
.chaine { color: #a3b58a; }
.nombre { color: #d9a066; }
.commentaire { color: #6f6c66; }
.interp { color: #e3c77f; }
.fonction { color: #e0cfa8; }
.ponct { color: #8a877f; }
