/* Azure Docs : theme sombre et sobre. Gris chauds, un seul accent (sable),
   pas de bleu. Fond #121212, surfaces #181818 / #1e1e1e, filets #272727,
   texte #e7e5e1 / #b8b5ae / #8a877f, accent #c9a878. */

.app { display: flex; flex-direction: column; height: 100%; background-color: #121212; color: #e7e5e1; font-size: 15px; scrollbar-color: #3b3935; }

/* ------------------------------------------------------------ haut */
.haut { display: flex; flex-direction: row; align-items: center; gap: 10px; height: 58px; padding: 0 20px; flex-shrink: 0; background-color: #161616; border-bottom: 1px solid #272727; }
.marque { padding: 6px 4px; background-color: transparent; color: #e7e5e1; font-size: 17px; font-weight: 700; cursor: pointer; }
.marque-sous { color: #6f6c66; font-size: 13px; }
.haut-droite { display: flex; flex-direction: row; align-items: center; gap: 8px; margin-left: auto; }
.champ { width: 380px; height: 36px; padding: 0 12px; background-color: #1e1e1e; border: 1px solid #2c2c2c; border-radius: 8px; color: #e7e5e1; font-size: 14px; }
.champ:focus { border: 1px solid #c9a878; }
.chercher { height: 36px; padding: 0 16px; background-color: #262420; border: 1px solid #3a3530; border-radius: 8px; color: #e0cfb3; font-size: 14px; font-weight: 600; cursor: pointer; transition: background-color 0.15s ease; }
.chercher:hover { background-color: #302c26; }

.corps { display: flex; flex-direction: row; flex-grow: 1; min-height: 0; }

/* ------------------------------------------------------------ menu lateral */
.cote { display: flex; flex-direction: column; gap: 2px; width: 268px; flex-shrink: 0; padding: 16px 12px 24px 12px; overflow-y: auto; background-color: #151515; border-right: 1px solid #242424; }
.cote-accueil { padding: 8px 12px; margin-bottom: 8px; background-color: transparent; border-radius: 6px; color: #b8b5ae; font-size: 14px; font-weight: 600; text-align: left; cursor: pointer; transition: background-color 0.15s ease; }
.cote-accueil:hover { background-color: #1e1e1e; }
.cote-accueil.on { background-color: #1f1d1a; color: #e0cfb3; }
.cote-section { padding: 8px 12px; background-color: transparent; border-radius: 6px; color: #9d9a93; font-size: 14px; text-align: left; cursor: pointer; transition: background-color 0.15s ease; }
.cote-section:hover { background-color: #1e1e1e; color: #e7e5e1; }
.cote-section.on { color: #e7e5e1; font-weight: 600; }
.cote-pages { display: flex; flex-direction: column; gap: 1px; margin: 2px 0 10px 14px; padding-left: 10px; border-left: 1px solid #2a2a2a; }
.cote-page { padding: 6px 10px; background-color: transparent; border-radius: 6px; color: #8a877f; font-size: 13px; text-align: left; cursor: pointer; transition: background-color 0.15s ease; }
.cote-page:hover { background-color: #1e1e1e; color: #d6d3cc; }
.cote-page.on { background-color: #1f1d1a; color: #e0cfb3; font-weight: 600; }

/* ------------------------------------------------------------ zone principale */
.principal { display: flex; flex-direction: column; flex-grow: 1; min-width: 0; overflow-y: auto; }
.article { display: flex; flex-direction: column; gap: 12px; max-width: 860px; padding: 32px 56px 64px 56px; }
.large { display: flex; flex-direction: column; gap: 14px; padding: 36px 48px 64px 48px; }

/* fil d'ariane */
.fil { display: flex; flex-direction: row; align-items: center; gap: 6px; }
.fil-lien { padding: 2px 0; background-color: transparent; color: #8a877f; font-size: 13px; cursor: pointer; }
.fil-lien:hover { color: #e0cfb3; }
.fil-sep { color: #4d4a45; font-size: 13px; }
.fil-ici { color: #b8b5ae; font-size: 13px; }

.titre { color: #f0eeea; font-size: 30px; font-weight: 700; margin-top: 6px; }
.resume { color: #9d9a93; font-size: 16px; line-height: 1.5; }
.filet { height: 1px; margin: 10px 0 4px 0; flex-shrink: 0; background-color: #262626; }

/* ------------------------------------------------------------ blocs de page */
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
.apercu { display: flex; flex-direction: column; margin-top: -13px; border: 1px solid #272727; border-top: 0; border-radius: 0 0 8px 8px; }
.apercu-tete { display: flex; flex-direction: row; align-items: center; gap: 10px; padding: 7px 14px; background-color: #151515; border-top: 1px dashed #2a2825; }
.apercu-titre { color: #c9a878; font-size: 11px; font-weight: 700; letter-spacing: 1px; }
.apercu-legende { color: #9d9a93; font-size: 12px; }
.apercu-note { margin-left: auto; color: #5f5c56; font-size: 11px; }
.apercu-cadre { display: flex; flex-direction: column; gap: 8px; padding: 18px; background-color: #0d0d0d; border-radius: 0 0 8px 8px; color: #c9c6bf; font-size: 14px; }
.demo { display: flex; flex-direction: row; align-items: center; gap: 18px; padding: 16px 18px; background-color: #1a1917; border: 1px solid #2f2b25; border-radius: 10px; }
.demo-texte { display: flex; flex-direction: column; gap: 4px; flex-grow: 1; min-width: 0; }
.demo-titre { color: #c9a878; font-size: 12px; font-weight: 700; letter-spacing: 1px; }
.demo-desc { color: #c9c6bf; font-size: 14px; line-height: 1.55; }
.demo-ouvrir { flex-shrink: 0; height: 36px; padding: 0 20px; background-color: #c9a878; border-radius: 8px; color: #1a1712; font-size: 14px; font-weight: 700; cursor: pointer; transition: background-color 0.15s ease; }
.demo-ouvrir:hover { background-color: #d8b98a; }
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

/* precedent / suivant */
.voisines { display: flex; flex-direction: row; gap: 14px; margin-top: 36px; }
.voisine, .voisine-vide { display: flex; flex-direction: column; gap: 4px; flex-grow: 1; flex-basis: 0; padding: 14px 16px; }
.voisine { position: relative; border: 1px solid #272727; border-radius: 8px; }
.voisine.droite { align-items: flex-end; }
.voisine-l { color: #6f6c66; font-size: 12px; }
.voisine-t { color: #e0cfb3; font-size: 15px; font-weight: 600; }

/* bouton transparent pose sur toute une carte (les boutons ne contiennent
   que du texte) : c'est lui qui recoit le clic. */
.voile { position: absolute; top: 0px; left: 0px; right: 0px; bottom: 0px; background-color: transparent; border-radius: 8px; cursor: pointer; transition: background-color 0.15s ease; }
.voile:hover { background-color: rgba(255, 255, 255, 0.025); }

/* ------------------------------------------------------------ accueil */
.intro { display: flex; flex-direction: column; gap: 12px; padding-bottom: 8px; }
.surtitre { color: #c9a878; font-size: 13px; font-weight: 600; letter-spacing: 2px; }
.intro-titre { color: #f0eeea; font-size: 36px; font-weight: 700; }
.intro-texte { max-width: 760px; color: #9d9a93; font-size: 16px; line-height: 1.6; }
.chiffres { display: flex; flex-direction: row; gap: 12px; margin-top: 6px; }
.chiffre { display: flex; flex-direction: row; align-items: center; gap: 8px; padding: 8px 14px; background-color: #181818; border: 1px solid #262626; border-radius: 8px; }
.chiffre-n { color: #e0cfb3; font-size: 16px; font-weight: 700; }
.chiffre-l { color: #8a877f; font-size: 13px; }

.bloc-titre { color: #8a877f; font-size: 12px; font-weight: 700; letter-spacing: 2px; margin-top: 18px; }
.parcours { display: flex; flex-direction: column; gap: 12px; }
.parcours-ligne { display: flex; flex-direction: row; gap: 14px; }
.etape { position: relative; display: flex; flex-direction: column; gap: 6px; flex-grow: 1; flex-basis: 0; padding: 16px 18px; background-color: #181818; border: 1px solid #262626; border-radius: 8px; }
.etape-n { color: #c9a878; font-family: monospace; font-size: 13px; }
.etape-t { color: #f0eeea; font-size: 16px; font-weight: 600; }
.etape-d { color: #8a877f; font-size: 13px; line-height: 1.5; }

.grille { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 14px; }
.carte { display: flex; flex-direction: column; gap: 8px; padding: 16px 18px; background-color: #181818; border: 1px solid #262626; border-radius: 8px; }
.carte-tete { display: flex; flex-direction: row; align-items: center; }
.carte-num { color: #c9a878; font-family: monospace; font-size: 13px; }
.carte-compte { margin-left: auto; color: #6f6c66; font-size: 12px; }
.carte-titre { padding: 0; background-color: transparent; color: #f0eeea; font-size: 18px; font-weight: 700; text-align: left; cursor: pointer; }
.carte-titre:hover { color: #e0cfb3; }
.carte-resume { color: #9d9a93; font-size: 13px; line-height: 1.5; }
.carte-pages { display: flex; flex-direction: row; flex-wrap: wrap; gap: 6px; margin-top: 4px; }
.carte-page { padding: 3px 9px; background-color: #1f1f1f; border: 1px solid #2a2a2a; border-radius: 5px; color: #b8b5ae; font-size: 12px; cursor: pointer; transition: background-color 0.15s ease; }
.carte-page:hover { background-color: #26241f; color: #e0cfb3; }

/* ------------------------------------------------------------ section */
.sommaire { display: flex; flex-direction: column; gap: 8px; margin-top: 12px; }
.ligne { position: relative; display: flex; flex-direction: row; align-items: center; gap: 16px; padding: 14px 16px; background-color: #181818; border: 1px solid #262626; border-radius: 8px; }
.ligne-num { width: 28px; flex-shrink: 0; color: #c9a878; font-family: monospace; font-size: 14px; }
.ligne-texte { display: flex; flex-direction: column; gap: 3px; flex-grow: 1; min-width: 0; }
.ligne-titre { color: #f0eeea; font-size: 16px; font-weight: 600; }
.ligne-resume { color: #8a877f; font-size: 13px; line-height: 1.45; }
.ligne-compte { flex-shrink: 0; color: #6f6c66; font-size: 12px; }

/* ------------------------------------------------------------ recherche */
.resultats { display: flex; flex-direction: column; gap: 8px; margin-top: 10px; }
.resultat { position: relative; display: flex; flex-direction: column; gap: 5px; padding: 12px 16px; background-color: #181818; border: 1px solid #262626; border-radius: 8px; }
.resultat-tete { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.resultat-genre { padding: 1px 8px; background-color: #222222; border-radius: 4px; color: #9d9a93; font-size: 11px; font-weight: 600; }
.resultat-genre.exemple { background-color: #1f1d1a; color: #c9a878; }
.resultat-id { color: #b8b5ae; font-family: monospace; font-size: 12px; }
.resultat-lieu { margin-left: auto; color: #6f6c66; font-size: 12px; }
.resultat-titre { color: #f0eeea; font-size: 15px; font-weight: 600; }
.resultat-extrait { color: #8a877f; font-size: 13px; line-height: 1.45; }

.erreur { color: #e0ad94; font-family: monospace; font-size: 13px; white-space: pre-wrap; }
