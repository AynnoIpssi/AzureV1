/* Azure Testeur : le theme d'Azure Note - gris chauds, accent sable,
   sauge pour ce qui passe, terracotta pour ce qui echoue, pas de bleu. */
.app { display: flex; flex-direction: column; height: 100%; background-color: #1f1e1c; color: #d8d5ce; font-size: 13px; scrollbar-color: #4a4843; accent-color: #c9a878; }

/* Barre du haut. */
.haut { display: flex; flex-direction: row; align-items: center; gap: 6px; height: 46px; padding: 0 14px; flex-shrink: 0; background-color: #252422; border-bottom: 1px solid #34322f; }
.logo { width: 18px; height: 18px; background: linear-gradient(135deg, #9bb08f 0%, #c9a878 100%); border-radius: 5px; flex-shrink: 0; }
.marque { color: #f0eeea; font-size: 14px; font-weight: 700; margin-right: 18px; flex-shrink: 0; }
.onglet { height: 28px; padding: 0 12px; background-color: transparent; border-radius: 6px; color: #9c998f; font-size: 13px; font-weight: 500; cursor: pointer; }
.onglet:hover { background-color: rgba(255, 255, 255, 0.05); color: #f0eeea; }
.onglet.on { background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; font-weight: 600; }
.etat-exec { display: flex; flex-direction: row; align-items: center; gap: 8px; margin-left: auto; min-width: 0; }
.point-vivant { width: 8px; height: 8px; border-radius: 4px; background-color: #e3bf7a; flex-shrink: 0; }
.exec { color: #aeaba3; font-size: 12px; }
.exec.ko { color: #e0ad94; }

.alerte { display: flex; flex-direction: row; align-items: center; gap: 12px; padding: 7px 18px; flex-shrink: 0; }
.alerte.ko { background-color: rgba(201, 138, 107, 0.14); border-bottom: 1px solid rgba(201, 138, 107, 0.35); }
.alerte.ok { background-color: rgba(155, 176, 143, 0.12); border-bottom: 1px solid rgba(155, 176, 143, 0.3); }
.alerte-texte { flex-grow: 1; min-width: 0; color: #e7e3da; font-size: 13px; }
.alerte-x { height: 22px; padding: 0 10px; background-color: transparent; border-radius: 5px; color: #aeaba3; font-size: 12px; cursor: pointer; }
.alerte-x:hover { background-color: rgba(255, 255, 255, 0.07); }

.corps { display: flex; flex-direction: row; flex-grow: 1; min-height: 0; }

/* Projets. */
.cote { display: flex; flex-direction: column; gap: 4px; width: 230px; flex-shrink: 0; padding: 14px 8px 10px 8px; background-color: #232220; border-right: 1px solid #34322f; }
.cote-titre { padding: 0 8px 2px 8px; color: #8c8981; font-size: 10.5px; font-weight: 700; letter-spacing: 1px; }
.cote-titre.espace { margin-top: 14px; }
.projets { display: flex; flex-direction: column; gap: 1px; flex-shrink: 1; min-height: 0; overflow-y: auto; }
.projet { display: flex; flex-direction: row; align-items: center; gap: 4px; height: 30px; border-radius: 6px; flex-shrink: 0; }
.projet:hover { background-color: rgba(255, 255, 255, 0.04); }
.projet-nom { flex-grow: 1; min-width: 0; height: 30px; padding: 0 8px; background-color: transparent; border-radius: 6px; color: #c9c6bf; font-size: 13px; text-align: left; cursor: pointer; }
.projet-nom.on { background-color: rgba(201, 168, 120, 0.13); color: #f0eeea; font-weight: 600; }
.projet-env { padding: 1px 7px; margin-right: 6px; border-radius: 999px; background-color: rgba(155, 176, 143, 0.14); color: #b9cbb0; font-size: 10.5px; font-weight: 600; }
.projet-x { width: 22px; height: 22px; padding: 0; margin-right: 4px; background-color: transparent; border-radius: 5px; color: #6f6c66; font-size: 13px; cursor: pointer; }
.projet-x:hover { background-color: rgba(201, 138, 107, 0.18); color: #e0ad94; }
.cote-champ { height: 28px; margin: 0 6px; padding: 0 8px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12px; }
.cote-champ:focus { border: 1px solid #c9a878; }
.cote-bouton { height: 28px; margin: 2px 6px 0 6px; background-color: #2e2c29; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12px; font-weight: 600; cursor: pointer; }
.cote-bouton:hover { background-color: #37342f; }
.cote-aide { padding: 2px 8px; color: #6f6c66; font-size: 11px; }

.principal { display: flex; flex-direction: column; flex-grow: 1; min-width: 0; min-height: 0; }
.vide { display: flex; flex-direction: column; gap: 8px; padding: 60px 48px; }
.vide-titre { color: #f0eeea; font-size: 22px; font-weight: 700; }
.vide-texte { color: #8c8981; font-size: 13px; }

/* Boutons. */
.bouton { height: 28px; padding: 0 12px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12.5px; font-weight: 500; cursor: pointer; flex-shrink: 0; }
.bouton:hover { background-color: #34322e; }
.bouton.fort { background-color: #c9a878; border: 1px solid #c9a878; color: #1a1712; font-weight: 700; }
.bouton.fort:hover { background-color: #d8b98a; }
.bouton.danger { background-color: rgba(201, 138, 107, 0.16); border: 1px solid rgba(201, 138, 107, 0.4); color: #e0ad94; }
.petit { height: 24px; padding: 0 10px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #c9c6bf; font-size: 12px; cursor: pointer; flex-shrink: 0; }
.petit:hover { background-color: #34322e; }
.petit.on { background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; }
.petit.danger { background-color: rgba(201, 138, 107, 0.16); border: 1px solid rgba(201, 138, 107, 0.4); color: #e0ad94; }
.mini { height: 22px; padding: 0 8px; background-color: transparent; border: 1px solid #3a3834; border-radius: 5px; color: #c9c6bf; font-size: 11.5px; cursor: pointer; flex-shrink: 0; }
.mini:hover { background-color: rgba(255, 255, 255, 0.06); color: #f0eeea; }
.mini.discret { border: 1px solid transparent; color: #8c8981; }
.rangee { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.pousse { flex-grow: 1; }

/* Onglet Tests. */
.tests { display: flex; flex-direction: column; gap: 12px; flex-grow: 1; min-height: 0; padding: 20px 24px 0 24px; }
.entete { display: flex; flex-direction: row; align-items: center; gap: 8px; flex-shrink: 0; }
.entete-texte { display: flex; flex-direction: column; gap: 2px; flex-grow: 1; min-width: 0; }
.h { color: #f3f1ec; font-size: 22px; font-weight: 700; }
.sous { color: #8c8981; font-size: 12px; }

.chiffres { display: flex; flex-direction: row; gap: 10px; flex-shrink: 0; }
.chiffre { display: flex; flex-direction: column; gap: 1px; width: 120px; padding: 10px 14px; background-color: #252422; border: 1px solid #312f2c; border-radius: 8px; }
.chiffre-n { color: #f0eeea; font-size: 20px; font-weight: 700; }
.chiffre-l { color: #8c8981; font-size: 11.5px; }
.chiffre.ok .chiffre-n { color: #b9cbb0; }
.chiffre.ko .chiffre-n { color: #e0ad94; }

.outils { display: flex; flex-direction: row; align-items: center; gap: 8px; flex-shrink: 0; }
.filtre { width: 320px; height: 28px; padding: 0 10px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12.5px; }
.filtre:focus { border: 1px solid #c9a878; }
.vues { display: flex; flex-direction: row; gap: 2px; padding: 2px; margin: 0 6px; background-color: #252422; border-radius: 7px; }
.vue { height: 24px; padding: 0 10px; background-color: transparent; border-radius: 5px; color: #9c998f; font-size: 12px; cursor: pointer; }
.vue:hover { color: #f0eeea; }
.vue.on { background-color: #36342f; color: #f0eeea; font-weight: 600; }

.zone { display: flex; flex-direction: row; gap: 16px; flex-grow: 1; min-height: 0; }
.liste { display: flex; flex-direction: column; gap: 6px; flex-grow: 1; min-width: 0; padding-bottom: 20px; overflow-y: auto; }
.groupe { display: flex; flex-direction: column; flex-shrink: 0; background-color: #232220; border: 1px solid #2f2d2a; border-radius: 8px; }
.groupe-tete { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 34px; padding: 0 8px 0 10px; }
.pli { width: 12px; color: #8c8981; font-family: monospace; font-size: 13px; flex-shrink: 0; }
.groupe-nom { height: 26px; padding: 0 4px; background-color: transparent; border-radius: 5px; color: #ece8e0; font-size: 13px; font-weight: 600; cursor: pointer; flex-shrink: 0; }
.groupe-nom:hover { color: #e0cfb3; }
.groupe-dossier { flex-grow: 1; min-width: 0; color: #6f6c66; font-size: 11.5px; }
.groupe-nb { color: #8c8981; font-size: 11.5px; }
.pastille { padding: 1px 8px; border-radius: 999px; background-color: rgba(255, 255, 255, 0.06); color: #aeaba3; font-size: 11px; flex-shrink: 0; }
.pastille.ok { background-color: rgba(155, 176, 143, 0.14); color: #b9cbb0; }
.pastille.ko { background-color: rgba(201, 138, 107, 0.16); color: #e0ad94; }
.pastille.attente { background-color: rgba(214, 170, 90, 0.14); color: #e3bf7a; }

.ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 30px; padding: 0 8px 0 30px; border-top: 1px solid #2b2a27; }
.ligne:hover { background-color: rgba(255, 255, 255, 0.025); }
.point { width: 8px; height: 8px; border-radius: 4px; background-color: #4a4843; flex-shrink: 0; }
.point.reussi { background-color: #9bb08f; }
.point.echoue { background-color: #d9876a; }
.point.erreur { background-color: #d9876a; }
.point.ignore { background-color: #6f6c66; }
.point.en_cours { background-color: #e3bf7a; }
.nom-test { flex-grow: 1; min-width: 0; height: 26px; padding: 0 4px; background-color: transparent; border-radius: 5px; color: #d8d5ce; font-family: monospace; font-size: 12px; text-align: left; cursor: pointer; }
.nom-test:hover { color: #f0eeea; }
.nom-test.on { color: #e0cfb3; font-weight: 700; }
.etat { width: 80px; color: #6f6c66; font-size: 11.5px; flex-shrink: 0; }
.etat.reussi { color: #b9cbb0; }
.etat.echoue { color: #e0ad94; }
.etat.erreur { color: #e0ad94; }
.etat.en_cours { color: #e3bf7a; }
.avert { color: #e3bf7a; font-size: 12px; }

/* Test choisi. */
.detail { display: flex; flex-direction: column; gap: 8px; width: 400px; flex-shrink: 0; padding: 16px; margin-bottom: 20px; background-color: #252422; border: 1px solid #312f2c; border-radius: 10px; overflow-y: auto; }
.detail-tete { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.detail-nom { flex-grow: 1; min-width: 0; color: #f3f1ec; font-family: monospace; font-size: 15px; font-weight: 700; }
.detail-etat { display: flex; flex-direction: row; align-items: center; gap: 8px; }
.etiquette { margin-top: 6px; color: #8c8981; font-size: 11px; font-weight: 700; letter-spacing: 0.5px; }
.sortie { display: flex; flex-direction: column; padding: 8px 10px; background-color: #1a1917; border: 1px solid rgba(201, 138, 107, 0.3); border-radius: 6px; flex-shrink: 0; }
.sortie-ligne { color: #e0ad94; font-family: monospace; font-size: 11.5px; }

/* Bloc de code : celui d'Azure Note. */
.code-bloc { display: flex; flex-direction: column; gap: 4px; min-width: 0; padding: 10px 12px; border-radius: 6px; background-color: #282725; border: 1px solid #34322f; flex-shrink: 0; }
.code-texte { min-height: 0px; padding: 0px; background-color: transparent; color: #e8e4dc; font-size: 12.5px; }

/* Sortie des commandes. */
.console { display: flex; flex-direction: column; height: 230px; flex-shrink: 0; background-color: #181715; border-top: 1px solid #34322f; }
.console-tete { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 32px; padding: 0 12px; flex-shrink: 0; border-bottom: 1px solid #2b2a27; }
.console-titre { flex-grow: 1; color: #aeaba3; font-size: 12px; font-weight: 600; }
.console-lignes { display: flex; flex-direction: column; flex-grow: 1; min-height: 0; padding: 6px 12px; overflow-y: auto; }
.console-ligne { color: #bdb9b0; font-family: monospace; font-size: 11.5px; flex-shrink: 0; }

/* Atelier. */
.atelier { display: flex; flex-direction: row; flex-grow: 1; min-height: 0; }
.fichiers { display: flex; flex-direction: column; gap: 4px; width: 280px; max-width: 280px; overflow-x: hidden; flex-shrink: 0; padding: 14px 8px; border-right: 1px solid #2f2d2a; }
.fichiers-liste { display: flex; flex-direction: column; gap: 1px; flex-shrink: 1; min-height: 0; overflow-y: auto; }
.fichier { min-width: 0; height: 28px; padding: 0 8px; background-color: transparent; border-radius: 6px; color: #aeaba3; font-family: monospace; font-size: 11.5px; text-align: left; cursor: pointer; flex-shrink: 0; }
.fichier:hover { background-color: rgba(255, 255, 255, 0.04); color: #f0eeea; }
.fichier.on { background-color: rgba(201, 168, 120, 0.13); color: #f0eeea; }
.ajout { height: 28px; margin-top: 6px; padding: 0 8px; background-color: transparent; border-radius: 6px; color: #c9a878; font-size: 12.5px; text-align: left; cursor: pointer; }
.ajout:hover { background-color: rgba(201, 168, 120, 0.1); }
.formulaire { display: flex; flex-direction: column; gap: 4px; padding: 10px 8px; margin-top: 4px; background-color: #252422; border-radius: 8px; }
.champ { height: 28px; padding: 0 8px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12px; }
.champ:focus { border: 1px solid #c9a878; }

.feuille { display: flex; flex-direction: column; flex-grow: 1; min-width: 0; min-height: 0; overflow-y: auto; }
.col-page { display: flex; flex-direction: column; gap: 8px; width: 100%; max-width: 820px; padding: 0 40px 60px 40px; box-sizing: border-box; align-self: center; }
.couverture { height: 70px; margin: 0 -40px 8px -40px; background: linear-gradient(135deg, #3a4636 0%, #4a3f30 100%); flex-shrink: 0; }
.page-titre { color: #f3f1ec; font-family: monospace; font-size: 20px; font-weight: 700; }
.section { margin-top: 18px; color: #f0eeea; font-size: 16px; font-weight: 700; }
.aide { color: #8c8981; font-size: 12px; }
.bloc-test { display: flex; flex-direction: column; gap: 8px; padding: 12px 14px; background-color: #232220; border: 1px solid #2f2d2a; border-radius: 8px; flex-shrink: 0; }
.bloc-test.nouveau { border: 1px dashed #4a4843; }
.nom-champ { flex-grow: 1; min-width: 0; height: 28px; padding: 0 8px; background-color: transparent; border: 1px solid transparent; border-radius: 6px; color: #f0eeea; font-family: monospace; font-size: 14px; font-weight: 700; }
.nom-champ:hover { border: 1px solid #3e3b36; }
.nom-champ:focus { border: 1px solid #c9a878; background-color: #2b2a27; }
.ligne-no { color: #6f6c66; font-size: 11.5px; flex-shrink: 0; }
.options { display: flex; flex-direction: row; flex-wrap: wrap; gap: 14px; }
.option { color: #aeaba3; font-size: 12px; }
.sans-alerte { height: 0px; flex-shrink: 0; }

/* Atelier : liste legere, un seul editeur ouvert. */
.ligne-bloc { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 36px; padding: 0 10px; background-color: #232220; border: 1px solid #2f2d2a; border-radius: 8px; flex-shrink: 0; }
.ligne-bloc:hover { background-color: #282725; border: 1px solid #3a3834; }
.ligne-titre { flex-grow: 1; min-width: 0; height: 30px; padding: 0 4px; background-color: transparent; border-radius: 5px; color: #e7e3da; font-family: monospace; font-size: 13px; text-align: left; cursor: pointer; }
.ligne-titre:hover { color: #e0cfb3; }
.ligne-titre.on { flex-grow: 0; color: #e0cfb3; font-weight: 700; }
.bloc-test.ouvert { border: 1px solid rgba(201, 168, 120, 0.35); }
.coupe { padding: 6px 10px; color: #e3bf7a; font-size: 12px; flex-shrink: 0; }

/* Composants (barre de gauche) : stockages > dossiers imbriques >
   composants, un retrait par niveau. */
.cote-entete { display: flex; flex-direction: row; align-items: center; margin-top: 14px; padding-right: 4px; flex-shrink: 0; }
.cote-plus { height: 22px; padding: 0 8px; background-color: transparent; border-radius: 5px; color: #c9a878; font-size: 11.5px; font-weight: 600; cursor: pointer; }
.cote-plus:hover { background-color: rgba(201, 168, 120, 0.12); }
.arbre { display: flex; flex-direction: column; gap: 1px; flex-shrink: 1; min-height: 0; overflow-y: auto; }
.arb-ligne { display: flex; flex-direction: row; align-items: center; gap: 4px; height: 26px; border-radius: 6px; flex-shrink: 0; }
.arb-ligne:hover { background-color: rgba(255, 255, 255, 0.04); }
.arb-p0 { padding-left: 2px; }
.arb-p1 { padding-left: 16px; }
.arb-p2 { padding-left: 30px; }
.arb-p3 { padding-left: 44px; }
.arb-p4 { padding-left: 58px; }
.arb-p5 { padding-left: 72px; }
.arb-p6 { padding-left: 86px; }
.arb-p7 { padding-left: 100px; }
.arb-p8 { padding-left: 114px; }
.arb-pli { width: 18px; height: 18px; padding: 0; background-color: transparent; border-radius: 4px; color: #8c8981; font-family: monospace; font-size: 12px; cursor: pointer; flex-shrink: 0; }
.arb-pli:hover { background-color: rgba(255, 255, 255, 0.07); color: #f0eeea; }
.arb-pli-vide { width: 18px; height: 18px; flex-shrink: 0; }
.arb-ic { width: 9px; height: 9px; flex-shrink: 0; border-radius: 2px; }
.arb-ic.stockage { background-color: #c9a878; border-radius: 3px; }
.arb-ic.dossier { border: 1.5px solid #9bb08f; }
.arb-ic.composant { width: 7px; height: 7px; border-radius: 4px; background-color: #8fa3b0; }
.arb-nom { flex-grow: 1; min-width: 0; height: 26px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #c9c6bf; font-size: 12.5px; text-align: left; cursor: pointer; }
.arb-nom.on { background-color: rgba(201, 168, 120, 0.13); color: #f0eeea; font-weight: 600; }

/* Page d'un composant, d'un dossier ou d'un stockage. */
.couverture.dossier { background: linear-gradient(135deg, #34402f 0%, #2d3a33 100%); }
.couverture.composant { background: linear-gradient(135deg, #2f3538 0%, #3a3530 100%); }
.fil { display: flex; flex-direction: row; align-items: center; gap: 2px; flex-wrap: wrap; }
.fil-lien { height: 22px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #8c8981; font-size: 12px; cursor: pointer; }
.fil-lien:hover { background-color: rgba(255, 255, 255, 0.05); color: #e0cfb3; }
.fil-sep { color: #5c5a55; font-size: 12px; }
.genre-pastille { padding: 1px 8px; border-radius: 999px; font-size: 11px; font-weight: 600; background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; flex-shrink: 0; }
.genre-pastille.dossier { background-color: rgba(155, 176, 143, 0.14); color: #b9cbb0; }
.genre-pastille.composant { background-color: rgba(143, 163, 176, 0.14); color: #b5c3cc; }
.titre-champ { flex-grow: 1; min-width: 0; height: 36px; padding: 0 8px; background-color: transparent; border: 1px solid transparent; border-radius: 6px; color: #f3f1ec; font-size: 20px; font-weight: 700; }
.titre-champ:hover { border: 1px solid #3e3b36; }
.titre-champ:focus { border: 1px solid #c9a878; background-color: #2b2a27; }
.champ.large { flex-grow: 1; min-width: 0; }
