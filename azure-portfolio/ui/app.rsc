/* Portfolio : editorial. Papier clair, encre, un seul accent (brique) pour
   les liens survoles. Titrage en serif (font-family: serif : la premiere
   serif du systeme), texte courant en Sora. Beaucoup d'air.
   Papier #f3efe7, encre #161513, gris #6e6a62, filet #d8d2c6, accent #9b3d25. */

.app { display: flex; flex-direction: column; height: 100%; background-color: #f3efe7; color: #2b2925; font-size: 17px; scrollbar-color: #c9c2b5; }

/* ------------------------------------------------------------ en-tete */
.haut { display: flex; flex-direction: row; align-items: center; gap: 28px; height: 76px; padding: 0 96px; flex-shrink: 0; background-color: #f3efe7; border-bottom: 1px solid #d8d2c6; }
.marque { margin-right: auto; color: #161513; font-family: serif; font-size: 21px; }
.nav { padding: 6px 0; background-color: transparent; color: #6e6a62; font-size: 14px; cursor: pointer; transition: background-color 0.2s ease; }
.nav:hover { color: #161513; }
.nav.on { color: #161513; text-decoration: underline; }

/* ------------------------------------------------------------ page */
.defile { display: flex; flex-direction: column; flex-grow: 1; min-height: 0; overflow-y: auto; }
.page { display: flex; flex-direction: column; padding: 0 96px 0 96px; }

/* ouverture : une grande phrase */
.ouverture { display: flex; flex-direction: column; gap: 36px; padding: 120px 0 110px 0; border-bottom: 1px solid #d8d2c6; }
.rubrique { color: #6e6a62; font-size: 12px; letter-spacing: 3px; }
.affiche { max-width: 920px; color: #161513; font-family: serif; font-size: 64px; line-height: 1.12; }
.affiche-2 { max-width: 920px; color: #161513; font-family: serif; font-size: 80px; line-height: 1.05; }
.chapeau { max-width: 640px; color: #4a4741; font-size: 20px; line-height: 1.6; }

/* sections : etiquette a gauche, texte a droite */
.bloc { display: flex; flex-direction: row; gap: 64px; padding: 88px 0; border-bottom: 1px solid #d8d2c6; }
.bloc-etiquette { width: 200px; flex-shrink: 0; padding-top: 10px; color: #6e6a62; font-size: 12px; letter-spacing: 3px; }
.bloc-corps { display: flex; flex-direction: column; gap: 22px; flex-grow: 1; flex-shrink: 1; min-width: 0; max-width: 720px; }
.titre { color: #161513; font-family: serif; font-size: 40px; line-height: 1.2; }
.titre-2 { color: #161513; font-family: serif; font-size: 28px; line-height: 1.3; }
.texte { color: #3a3834; font-size: 17px; line-height: 1.75; }
.grand-texte { color: #2b2925; font-family: serif; font-size: 26px; line-height: 1.55; }
.note { color: #6e6a62; font-size: 14px; line-height: 1.6; }
.sep { height: 1px; background-color: #d8d2c6; }

/* liens : du texte souligne, qui rougit au survol */
.lien { align-self: flex-start; padding: 2px 0; background-color: transparent; color: #161513; font-size: 16px; text-decoration: underline; cursor: pointer; transition: background-color 0.2s ease; }
.lien:hover { color: #9b3d25; }
.liens { display: flex; flex-direction: row; gap: 32px; flex-wrap: wrap; }

/* ------------------------------------------------------------ travaux */
.travail { display: flex; flex-direction: column; gap: 30px; padding: 96px 0; border-bottom: 1px solid #d8d2c6; }
.travail-tete { display: flex; flex-direction: row; align-items: flex-end; gap: 24px; }
.travail-titre { color: #161513; font-family: serif; font-size: 72px; line-height: 1.0; }
.travail-annee { margin-left: auto; padding-bottom: 10px; color: #6e6a62; font-size: 14px; }
.image { height: 560px; background-color: #1a1a1a; background-size: cover; background-position: top; border-radius: 3px; box-shadow: 0 14px 34px 0 rgba(40, 30, 20, 0.14); margin-bottom: 6px; }
.image.courte { height: 420px; }
.image.vide { display: flex; align-items: center; justify-content: center; background-color: #e7e1d6; box-shadow: none; }
.image-vide-texte { color: #9c968b; font-size: 14px; }
.legende { color: #6e6a62; font-size: 13px; }
.travail-texte { display: flex; flex-direction: row; gap: 64px; }
.travail-texte-gauche { width: 280px; flex-shrink: 0; display: flex; flex-direction: column; gap: 8px; }
.travail-texte-droite { display: flex; flex-direction: column; gap: 20px; flex-grow: 1; flex-shrink: 1; min-width: 0; max-width: 640px; }
.meta-nom { color: #6e6a62; font-size: 12px; letter-spacing: 2px; }
.meta { color: #2b2925; font-size: 15px; line-height: 1.6; }

.img-docs { background-image: url(ui/images/docs.png); }
.img-docs-rsc { background-image: url(ui/images/docs-rsc.png); }
.img-dashboard { background-image: url(ui/images/dashboard.png); }
.img-note { background-image: url(ui/images/note.png); }
.AutomatImport { background-image: url(ui/images/AutomatImport.png); }
.img-tests-maui { background-image: url(ui/images/tests-maui.png); }
.img-echecs { background-image: url(ui/images/echecs.png); }
.img-scratch { background-image: url(ui/images/scratch.png); }
.img-boutique { background-image: url(ui/images/boutique.png); }

/* deux images cote a cote */
.paire { display: flex; flex-direction: row; gap: 24px; }
.paire-col { display: flex; flex-direction: column; gap: 12px; flex-grow: 1; flex-basis: 0; min-width: 0; }

/* ------------------------------------------------------------ chiffres */
.chiffres { display: flex; flex-direction: row; gap: 0; }
.chiffre-bloc { display: flex; flex-direction: column; gap: 6px; flex-grow: 1; flex-basis: 0; padding: 0 28px; border-left: 1px solid #d8d2c6; }
.chiffre-bloc.premier { padding-left: 0; border-left: 0; }
.chiffre { color: #161513; font-family: serif; font-size: 52px; }

/* ------------------------------------------------------------ couches */
.couche { display: flex; flex-direction: row; gap: 24px; padding: 16px 0; border-top: 1px solid #d8d2c6; }
.couche-nom { width: 140px; flex-shrink: 0; color: #161513; font-family: serif; font-size: 19px; }
.couche-contenu { color: #4a4741; font-size: 15px; line-height: 1.6; }

/* ------------------------------------------------------------ parcours, veille */
.etape { display: flex; flex-direction: row; gap: 64px; padding: 64px 0; border-bottom: 1px solid #d8d2c6; }
.etape-date { width: 200px; flex-shrink: 0; padding-top: 12px; color: #6e6a62; font-size: 14px; }
.etape-corps { display: flex; flex-direction: column; gap: 14px; flex-grow: 1; flex-shrink: 1; min-width: 0; max-width: 720px; }
.lieu { color: #6e6a62; font-size: 15px; }

.citation { padding: 8px 0 8px 32px; border-left: 2px solid #161513; }
.citation-texte { color: #161513; font-family: serif; font-size: 30px; font-style: italic; line-height: 1.45; }

/* ------------------------------------------------------------ contact, pied */
.contact { display: flex; flex-direction: column; gap: 28px; padding: 120px 0; }
.contact-mail { color: #161513; font-family: serif; font-size: 56px; }
.pied { display: flex; flex-direction: row; justify-content: space-between; padding: 32px 96px; border-top: 1px solid #d8d2c6; }
.pied-texte { color: #8a857b; font-size: 13px; }
