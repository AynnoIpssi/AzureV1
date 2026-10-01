/* Azure Note v2 : le theme d'Azure - gris chauds, accent sable, pas de bleu. */
.app { display: flex; flex-direction: column; height: 100%; background-color: #1f1e1c; color: #d8d5ce; font-size: 13px; scrollbar-color: #4a4843; accent-color: #c9a878; }

/* Barre du haut : marque, chemin de la page, recherche dans la doc. */
.haut { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 44px; padding: 0 14px; flex-shrink: 0; background-color: #252422; border-bottom: 1px solid #34322f; }
.logo { width: 18px; height: 18px; background: linear-gradient(135deg, #c9a878 0%, #9c7b55 100%); border-radius: 5px; flex-shrink: 0; }
.marque { color: #f0eeea; font-size: 14px; font-weight: 700; flex-shrink: 0; }
.chemin { display: flex; flex-direction: row; align-items: center; gap: 2px; margin-left: 14px; min-width: 0; overflow-x: hidden; }
.chemin-sep { color: #67645e; font-size: 12px; }
.chemin-lien { height: 22px; padding: 0 5px; background-color: transparent; border-radius: 5px; color: #aeaba3; font-size: 12px; cursor: pointer; }
.chemin-lien:hover { background-color: rgba(255, 255, 255, 0.05); color: #e7e5e1; }
.doc { display: flex; flex-direction: row; align-items: center; gap: 8px; margin-left: auto; flex-shrink: 0; }
.doc-champ { width: 220px; height: 26px; padding: 0 10px; background-color: #2b2a27; border: 1px solid #3e3b36; border-radius: 6px; color: #e7e5e1; font-size: 12px; }
.doc-champ:focus { border: 1px solid #c9a878; }
.doc-bouton { height: 26px; padding: 0 12px; background-color: #c9a878; border-radius: 6px; color: #1a1712; font-size: 12px; font-weight: 700; cursor: pointer; }
.doc-bouton:hover { background-color: #d8b98a; }

.erreur { padding: 8px 18px; background-color: rgba(201, 138, 107, 0.14); border-bottom: 1px solid rgba(201, 138, 107, 0.35); flex-shrink: 0; }
.erreur-texte { color: #e0ad94; font-size: 13px; }

.corps { display: flex; flex-direction: row; flex-grow: 1; min-height: 0; }

/* Arbre des pages. */
.cote { display: flex; flex-direction: column; gap: 2px; width: 240px; flex-shrink: 0; padding: 12px 6px 8px 6px; background-color: #232220; border-right: 1px solid #34322f; }
.cote-tete { display: flex; flex-direction: row; align-items: center; padding: 0 4px 4px 10px; }
.cote-titre { color: #8c8981; font-size: 10.5px; font-weight: 700; letter-spacing: 1px; margin-right: auto; }
.cote-plus { width: 20px; height: 20px; padding: 0; background-color: transparent; border-radius: 4px; color: #8c8981; font-size: 14px; cursor: pointer; }
.cote-plus:hover { background-color: rgba(255, 255, 255, 0.07); color: #f0eeea; }
.liste { display: flex; flex-direction: column; gap: 1px; flex-grow: 1; min-height: 0; overflow-y: auto; }
.item { display: flex; flex-direction: row; align-items: center; gap: 4px; height: 28px; padding-right: 2px; border-radius: 6px; flex-shrink: 0; }
.item:hover { background-color: rgba(255, 255, 255, 0.045); }
.item.on { background-color: rgba(201, 168, 120, 0.13); }
.item.n0 { padding-left: 2px; }
.item.n1 { padding-left: 16px; }
.item.n2 { padding-left: 30px; }
.item.n3 { padding-left: 44px; }
.item.n4 { padding-left: 58px; }
.item.n5 { padding-left: 72px; }
.item.n6 { padding-left: 86px; }
.item-pli { width: 16px; height: 16px; padding: 0; flex-shrink: 0; background-color: transparent; border-radius: 4px; color: #9c998f; font-family: monospace; font-size: 13px; cursor: pointer; }
.item-pli:hover { background-color: rgba(255, 255, 255, 0.08); color: #f0eeea; }
.item-pli-vide { width: 16px; height: 16px; flex-shrink: 0; }
.item-nom { flex-grow: 1; min-width: 0; overflow-x: hidden; height: 24px; padding: 0 4px; background-color: transparent; color: #d2cfc8; font-size: 13px; text-align: left; cursor: pointer; }
.item-nom:hover { color: #f3f1ec; }
.item.on .item-nom { color: #f0e2c8; font-weight: 600; }
.item-act { width: 18px; height: 18px; padding: 0; flex-shrink: 0; background-color: transparent; border-radius: 4px; color: #6f6c66; font-size: 12px; cursor: pointer; }
.item-act:hover { background-color: rgba(255, 255, 255, 0.08); color: #f0eeea; }
.item-x:hover { color: #e0ad94; }
.item-confirmer { display: flex; flex-direction: column; gap: 6px; margin: 2px 4px 6px 4px; padding: 8px 10px; border-radius: 8px; background-color: rgba(201, 138, 107, 0.10); border: 1px solid rgba(201, 138, 107, 0.30); flex-shrink: 0; }
.item-question { color: #e8c2ae; font-size: 12px; }
.item-choix { display: flex; flex-direction: row; gap: 6px; justify-content: flex-end; }
.cote-bas { display: flex; flex-direction: column; gap: 1px; padding-top: 6px; border-top: 1px solid #34322f; flex-shrink: 0; }
.cote-lien { height: 28px; padding: 0 10px; background-color: transparent; border-radius: 6px; color: #9c998f; font-size: 12.5px; text-align: left; cursor: pointer; }
.cote-lien:hover { background-color: rgba(255, 255, 255, 0.05); color: #f0eeea; }

/* La page ouverte. */
.editeur { display: flex; flex-direction: column; flex-grow: 1; min-width: 0; }
.feuille { display: flex; flex-direction: column; align-items: center; gap: 0px; flex-grow: 1; min-height: 0; padding: 0 0 60px 0; overflow-y: auto; }
.titre { width: 100%; min-height: 0px; padding: 0px; margin-top: 6px; background-color: transparent; color: #f3f1ec; font-size: 30px; font-weight: 700; flex-shrink: 0; }
.meta-info { color: #aeaba3; font-size: 12px; }
.meta-question { color: #e0ad94; font-size: 13px; }

.btn { height: 24px; padding: 0 10px; background-color: #2c2b28; border: 1px solid #3e3b36; border-radius: 6px; color: #b8b5ae; font-size: 12px; font-weight: 600; cursor: pointer; }
.btn:hover { background-color: #35332f; color: #e7e5e1; }
.btn.primaire { background-color: #c9a878; border: 1px solid #c9a878; color: #1a1712; }
.btn.primaire:hover { background-color: #d8b98a; }
.btn.danger { background-color: rgba(201, 138, 107, 0.14); border: 1px solid rgba(201, 138, 107, 0.4); color: #e0ad94; }
.lien-discret { height: 24px; padding: 0 7px; background-color: transparent; border-radius: 5px; color: #9c998f; font-size: 12px; text-align: left; cursor: pointer; flex-shrink: 0; }
.lien-discret:hover { background-color: rgba(255, 255, 255, 0.05); color: #e7e5e1; }

/* Base : onglets des vues, filtres. */
.base { display: flex; flex-direction: column; gap: 10px; flex-shrink: 0; min-width: 0px; margin-top: 8px; }
.onglets { display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 4px; border-bottom: 1px solid #34322f; padding-bottom: 6px; }
.onglet { height: 24px; padding: 0 10px; background-color: transparent; border-radius: 5px; color: #aeaba3; font-size: 12px; font-weight: 600; cursor: pointer; }
.onglet:hover { background-color: rgba(255, 255, 255, 0.05); }
.onglet.on { background-color: rgba(201, 168, 120, 0.14); color: #e0cfb3; }
.onglets-fin { display: flex; flex-direction: row; flex-wrap: wrap; gap: 2px; margin-left: auto; }
.filtres { display: flex; flex-direction: column; gap: 8px; padding: 10px; border-radius: 8px; background-color: #2a2926; }
.filtres-ligne { display: flex; flex-direction: row; align-items: center; gap: 6px; }
.filtres-titre { width: 60px; color: #9c998f; font-size: 12px; }
.f-prop { width: 170px; }
.f-op { width: 130px; }
.f-val { width: 150px; height: 26px; padding: 0 8px; background-color: #2e2d2a; border-radius: 5px; color: #e7e5e1; font-size: 12px; }
.puce-filtre { display: flex; flex-direction: row; align-items: center; gap: 4px; height: 26px; padding: 0 4px 0 10px; border-radius: 13px; background-color: rgba(201, 168, 120, 0.12); }
.puce-texte { color: #e0cfb3; font-size: 12px; }
.puce-x { width: 16px; height: 16px; padding: 0; background-color: transparent; border-radius: 8px; color: #c9a878; font-size: 11px; cursor: pointer; }

/* Vue table. */
.table { display: flex; flex-direction: column; border: 1px solid #34322f; border-radius: 8px; overflow-x: auto; }
.table-tete { display: flex; flex-direction: row; align-items: center; height: 28px; background-color: #2a2926; border-bottom: 1px solid #34322f; }
.th { display: flex; flex-direction: row; align-items: center; width: 160px; flex-shrink: 0; padding: 0 4px 0 10px; }
.th-nom { width: 200px; flex-shrink: 0; padding: 0 10px; color: #9c998f; font-size: 11px; font-weight: 700; }
.th-icone { width: 16px; color: #6f6c66; font-family: monospace; font-size: 12px; flex-shrink: 0; }
.th-texte { flex-grow: 1; min-width: 0; height: 24px; padding: 0 4px; border-radius: 4px; background-color: transparent; color: #9c998f; font-size: 11px; font-weight: 700; text-align: left; cursor: pointer; }
.th-texte:hover { background-color: rgba(255, 255, 255, 0.05); color: #e8e4dc; }
.th.on .th-texte { color: #e3c99d; }
.th-plus { width: 28px; height: 28px; padding: 0; background-color: transparent; color: #8c8981; font-size: 13px; cursor: pointer; }
.th-plus:hover { color: #e7e5e1; }
.table-corps { display: flex; flex-direction: column; }
.tr { display: flex; flex-direction: row; align-items: center; min-height: 30px; border-bottom: 1px solid #2e2d2a; }
.tr:hover { background-color: rgba(255, 255, 255, 0.02); }
.td { width: 160px; min-width: 0px; flex-shrink: 0; padding: 0 4px; overflow-x: hidden; }
.td-nom { width: 200px; }
.th { min-width: 0px; overflow-x: hidden; }
.ligne-nom { width: 100%; min-width: 0px; overflow-x: hidden; height: 26px; padding: 0 8px; background-color: transparent; color: #e7e5e1; font-size: 12.5px; font-weight: 600; text-align: left; cursor: pointer; }
.ligne-nom:hover { color: #e0cfb3; }

/* Vue kanban : colonnes (zones de depot), cartes (a glisser). */
.kanban { display: flex; flex-direction: row; align-items: flex-start; gap: 12px; overflow-x: auto; padding-bottom: 8px; }
.colonne { display: flex; flex-direction: column; gap: 8px; width: 250px; flex-shrink: 0; padding: 10px; border-radius: 10px; background-color: #282725; min-height: 120px; }
.col-tete { display: flex; flex-direction: row; align-items: center; gap: 8px; padding: 0 4px; }
.col-nom { color: #e0cfb3; font-size: 12px; font-weight: 700; }
.col-nb { color: #8c8981; font-size: 12px; }
.carte { display: flex; flex-direction: column; gap: 4px; padding: 10px; border-radius: 8px; background-color: #302e2b; border: 1px solid #383632; cursor: grab; }
.carte:hover { border: 1px solid #4c473f; }
.carte-nom { height: 20px; padding: 0; background-color: transparent; color: #eeeae2; font-size: 12.5px; font-weight: 600; text-align: left; cursor: pointer; }
.carte-detail { color: #9c998f; font-size: 11px; }
.col-plus { height: 28px; background-color: transparent; border-radius: 6px; color: #8c8981; font-size: 13px; text-align: left; cursor: pointer; }
.col-plus:hover { background-color: rgba(255, 255, 255, 0.04); color: #e7e5e1; }

/* Vues liste et galerie. */
.liste-base { display: flex; flex-direction: column; }
.ligne-liste { display: flex; flex-direction: row; align-items: center; gap: 12px; min-height: 36px; border-bottom: 1px solid #2e2d2a; }
.ligne-liste .ligne-nom { width: 260px; flex-shrink: 0; }
.ligne-detail { color: #9c998f; font-size: 12px; }
.galerie { display: flex; flex-direction: row; flex-wrap: wrap; gap: 12px; }
.vignette { display: flex; flex-direction: column; gap: 6px; width: 220px; min-height: 110px; padding: 12px; border-radius: 10px; background-color: #2b2a27; border: 1px solid #383632; }
.vignette-plus { width: 220px; height: 110px; border-radius: 10px; background-color: transparent; border: 1px dashed #3e3b36; color: #8c8981; cursor: pointer; }

/* Blocs. */
.blocs { display: flex; flex-direction: column; gap: 1px; flex-shrink: 0; margin-top: 12px; margin-left: -18px; }
.bloc { display: flex; flex-direction: row; align-items: flex-start; gap: 6px; border-radius: 4px; padding: 3px 0; }
.bloc:hover { background-color: rgba(255, 255, 255, 0.025); }
.poignee { width: 12px; flex-shrink: 0; padding-top: 5px; color: #4a4843; font-size: 9px; font-weight: 700; cursor: grab; }
.marque-liste { width: 16px; flex-shrink: 0; padding-top: 2px; color: #c9a878; font-size: 13px; text-align: right; }
.tache { margin-top: 3px; flex-shrink: 0; }
.bloc-texte { flex-grow: 1; min-width: 0; min-height: 0px; padding: 0px; background-color: transparent; color: #e8e4dc; font-size: 14px; }
.bloc-texte.titre1 { font-size: 22px; font-weight: 700; color: #f0eeea; }
.bloc-texte.titre2 { font-size: 18px; font-weight: 700; color: #ece8e0; }
.bloc-texte.titre3 { font-size: 15px; font-weight: 700; color: #e7e3da; }
.bloc-texte.citation { color: #b8b3a8; border-left: 3px solid #c9a878; }
.bloc.titre1 { margin-top: 14px; }
.bloc.titre2 { margin-top: 10px; }
.bloc-genre { width: 130px; flex-shrink: 0; opacity: 0.45; }
.bloc-genre:hover { opacity: 1; }
.bloc-x { width: 16px; height: 16px; margin-top: 3px; padding: 0; flex-shrink: 0; background-color: transparent; border-radius: 4px; color: #4f4d48; font-size: 11px; cursor: pointer; }
.bloc-x:hover { color: #e0ad94; background-color: rgba(255, 255, 255, 0.05); }
/* Poignee et × : seulement au survol du bloc. */
.poignee, .bloc-x { opacity: 0; }
.bloc:hover .poignee, .bloc:hover .bloc-x { opacity: 1; }

.code-bloc { display: flex; flex-direction: column; gap: 4px; flex-grow: 1; min-width: 0; padding: 10px 12px; border-radius: 6px; background-color: #282725; border: 1px solid transparent; }
.code-langage { width: 120px; height: 18px; padding: 0 2px; background-color: transparent; border: 1px solid transparent; color: #8c8981; font-size: 11px; }
.code-texte { min-height: 0px; padding: 0px; background-color: transparent; color: #e8e4dc; font-size: 12.5px; }

.tableau { display: flex; flex-direction: column; flex-grow: 1; min-width: 0; }
.tab-ligne { display: flex; flex-direction: row; }
.entete, .cellule { min-width: 0px; width: 150px; height: 26px; padding: 0 6px; border: 1px solid #34322f; border-radius: 0; background-color: transparent; color: #e8e4dc; font-size: 12px; }
.entete { background-color: #2b2a27; color: #e0cfb3; font-weight: 700; }
.tab-outils { display: flex; flex-direction: row; gap: 4px; margin-top: 4px; }

.trait { flex-grow: 1; height: 1px; margin: 10px 0 10px 18px; background-color: #3d3b37; }
.bloc.separateur { padding: 0; }
.bloc.separateur:hover { background-color: transparent; }
.lien-page { height: 24px; padding: 0 4px; background-color: transparent; border-radius: 4px; color: #e8e4dc; font-size: 14px; text-align: left; cursor: pointer; flex-shrink: 0; }
.lien-page:hover { background-color: rgba(201, 168, 120, 0.10); }

.vue-bloc { display: flex; flex-direction: column; gap: 4px; flex-grow: 1; min-width: 0; padding: 8px; border-radius: 8px; border: 1px solid #383632; }
.mini-table { display: flex; flex-direction: column; }
.mini-tr { display: flex; flex-direction: row; align-items: center; min-height: 28px; border-bottom: 1px solid #2e2d2a; }
.mini-tete { color: #8c8981; }
.mini-td { width: 150px; flex-shrink: 0; padding: 0 8px; color: #aeaba3; font-size: 12px; }
.mini-nom { width: 150px; flex-shrink: 0; height: 26px; padding: 0 8px; background-color: transparent; color: #e7e5e1; font-size: 12px; text-align: left; cursor: pointer; }

/* Barre « Ajouter ». */
.ajout { display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 4px; margin-top: 18px; padding-top: 12px; border-top: 1px solid #2e2d2a; flex-shrink: 0; }
.ajout-titre { color: #8c8981; font-size: 12px; margin-right: 6px; }
.ajout-btn { height: 28px; padding: 0 10px; background-color: #2b2a27; border: 1px solid #383632; border-radius: 6px; color: #b8b5ae; font-size: 12px; cursor: pointer; }
.ajout-btn:hover { background-color: #35332f; color: #f0eeea; }

.vide { display: flex; flex-direction: column; align-items: flex-start; gap: 10px; margin: auto; width: 480px; }
.vide-titre { color: #f0eeea; font-size: 24px; font-weight: 700; }
.vide-texte { color: #aeaba3; font-size: 14px; line-height: 1.55; }

/* Listes de choix plus discretes. */
.f-prop, .f-op { font-size: 12px; height: 26px; }

/* « + Ajouter un bloc » et le menu `/` (components/menu-blocs.rsh). */
.plus-bloc { height: 26px; margin-top: 6px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #7c7971; font-size: 12.5px; text-align: left; cursor: pointer; flex-shrink: 0; }
.plus-bloc:hover { background-color: rgba(255, 255, 255, 0.04); color: #b8b5ae; }
.menu-blocs { display: flex; flex-direction: column; width: 320px; margin-left: 20px; padding: 6px; border-radius: 8px; background-color: #2d2c29; border: 1px solid #3e3b36; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45); flex-shrink: 0; }
.menu-tete { display: flex; flex-direction: row; align-items: center; padding: 2px 4px 6px 8px; }
.menu-titre { color: #8c8981; font-size: 10px; font-weight: 700; letter-spacing: 1px; margin-right: auto; }
.menu-x { width: 16px; height: 16px; padding: 0; background-color: transparent; border-radius: 4px; color: #8c8981; font-size: 11px; cursor: pointer; }
.menu-liste { display: flex; flex-direction: column; max-height: 300px; overflow-y: auto; }
.menu-ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; border-radius: 5px; }
.menu-ligne:hover { background-color: rgba(201, 168, 120, 0.10); }
.menu-item { width: 120px; flex-shrink: 0; height: 28px; padding: 0 8px; background-color: transparent; color: #eeeae2; font-size: 12.5px; font-weight: 600; text-align: left; cursor: pointer; }
.menu-desc { color: #928f87; font-size: 11px; }

/* Options de la page, et la zone pour ecrire sous les blocs. */
.options { display: flex; flex-direction: column; gap: 6px; margin-top: 6px; padding: 10px 12px; border-radius: 8px; background-color: #282725; border: 1px solid #34322f; flex-shrink: 0; }
.rouge { color: #e0ad94; }
.zone-ecrire { min-height: 160px; flex-grow: 1; flex-shrink: 0; background-color: transparent; cursor: text; }

/* Identite de la page : couverture, pastille (initiale), puce dans l'arbre. */
.couverture { width: 100%; height: 120px; flex-shrink: 0; }
.col-page { display: flex; flex-direction: column; gap: 4px; box-sizing: border-box; width: 100%; max-width: 780px; min-width: 0px; margin: -30px auto 0 auto; padding: 0 56px; flex-shrink: 0; }
.col-page.moyenne { max-width: 1000px; }
.col-page.large { max-width: 1240px; }
.col-page.pleine { max-width: 100%; }
.entete-page { display: flex; flex-direction: row; align-items: flex-end; flex-shrink: 0; }
.pastille { width: 56px; height: 56px; padding: 0; border-radius: 12px; border: 3px solid #1f1e1c; color: #1f1e1c; font-size: 24px; font-weight: 700; cursor: pointer; }
.options-bouton { margin-left: auto; color: #8c8981; }
.options-ligne { display: flex; flex-direction: row; align-items: center; gap: 6px; }
.options-nom { width: 80px; color: #8c8981; font-size: 12px; }
.teinte { width: 18px; height: 18px; padding: 0; border-radius: 9px; border: 2px solid transparent; cursor: pointer; }
.teinte.on { border: 2px solid #f3f1ec; }
.choix-largeur { height: 24px; padding: 0 9px; background-color: transparent; border-radius: 5px; border: 1px solid #3a3834; color: #9c998f; font-size: 12px; cursor: pointer; flex-shrink: 0; }
.choix-largeur:hover { color: #e7e5e1; }
.choix-largeur.on { background-color: rgba(201, 168, 120, 0.14); border: 1px solid #c9a878; color: #e0cfb3; }
.puce-page { width: 8px; height: 8px; border-radius: 2px; flex-shrink: 0; }
.lien-ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; padding-left: 4px; flex-shrink: 0; }
.sous-pages { display: flex; flex-direction: column; gap: 2px; margin-top: 12px; padding-top: 10px; border-top: 1px solid #2e2d2a; flex-shrink: 0; }

.t-sable { background-color: #c9a878; }
.t-terre { background-color: #c98a6b; }
.t-olive { background-color: #a9b06e; }
.t-mousse { background-color: #7fae8e; }
.t-prune { background-color: #b384a8; }
.t-ardoise { background-color: #9690a8; }
.t-rose { background-color: #d49a9a; }
.t-ocre { background-color: #d6b35a; }
.couverture.t-sable { background: linear-gradient(120deg, #6b5a44 0%, #3a3128 100%); }
.couverture.t-terre { background: linear-gradient(120deg, #6e4a3a 0%, #3a2c26 100%); }
.couverture.t-olive { background: linear-gradient(120deg, #585c3a 0%, #30322a 100%); }
.couverture.t-mousse { background: linear-gradient(120deg, #3f5a4a 0%, #2a332e 100%); }
.couverture.t-prune { background: linear-gradient(120deg, #5c4058 0%, #33293a 100%); }
.couverture.t-ardoise { background: linear-gradient(120deg, #4c4a5c 0%, #2e2d36 100%); }
.couverture.t-rose { background: linear-gradient(120deg, #6e4a4a 0%, #3a2c2c 100%); }
.couverture.t-ocre { background: linear-gradient(120deg, #6e5a2e 0%, #3a3222 100%); }

/* Proprietes : des lignes nettes, le nom a gauche (un clic ouvre sa carte),
   la valeur a droite, a plat (un fond au survol seulement). */
.props { display: flex; flex-direction: column; gap: 1px; flex-shrink: 0; margin-top: 12px; padding-bottom: 12px; border-bottom: 1px solid #2e2d2a; }
.prop { display: flex; flex-direction: row; align-items: center; min-height: 32px; flex-shrink: 0; }
.prop-tete { display: flex; flex-direction: row; align-items: center; gap: 6px; width: 170px; flex-shrink: 0; }
.prop-icone { width: 18px; color: #8c8981; font-family: monospace; font-size: 15px; text-align: center; flex-shrink: 0; }
.prop-nom { flex-grow: 1; min-width: 0; overflow-x: hidden; height: 28px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #9c998f; font-size: 13px; font-weight: 500; text-align: left; cursor: pointer; }
.prop-nom:hover { background-color: rgba(255, 255, 255, 0.05); color: #e8e4dc; }
.prop-val { flex-grow: 1; min-width: 0; display: flex; flex-direction: row; align-items: center; }
.prop-ajout { height: 28px; margin-top: 2px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #6f6c66; font-size: 12.5px; text-align: left; cursor: pointer; flex-shrink: 0; }
.prop-ajout:hover { background-color: rgba(255, 255, 255, 0.04); color: #b8b5ae; }

/* Valeur d'une propriete (components/champ.rsh) : du texte, pas un formulaire. */
.ch-texte { width: 100%; height: 28px; padding: 0 6px; background-color: transparent; border: 1px solid transparent; border-radius: 5px; color: #e8e4dc; font-size: 13px; }
.ch-texte:hover { background-color: rgba(255, 255, 255, 0.04); }
.ch-texte:focus { background-color: #2a2926; border: 1px solid #45423c; }
.ch-case { margin-left: 6px; }
.ch-calc { padding: 0 6px; color: #e0cfb3; font-size: 13px; font-weight: 600; }
.ch-erreur { color: #e0ad94; font-weight: 400; }
.ch-bouton { height: 28px; padding: 0 6px; background-color: transparent; border-radius: 5px; color: #e8e4dc; font-size: 13px; text-align: left; cursor: pointer; flex-shrink: 0; }
.ch-bouton:hover { background-color: rgba(255, 255, 255, 0.05); }
.ch-bouton.true { background-color: rgba(255, 255, 255, 0.06); }
.ch-vide { color: #6f6c66; }
.ch-pastilles { display: flex; flex-direction: row; flex-wrap: wrap; align-items: center; gap: 4px; min-width: 0; padding: 3px 4px; }

/* Une option, une etiquette, une page liee : une pastille coloree. */
.opt { height: 22px; padding: 0 8px; border-radius: 4px; background-color: rgba(255, 255, 255, 0.08); color: #e8e4dc; font-size: 12px; font-weight: 500; flex-shrink: 0; cursor: pointer; }
.opt.c0 { background-color: rgba(201, 168, 120, 0.18); color: #e3c99d; }
.opt.c1 { background-color: rgba(217, 140, 106, 0.18); color: #eeb296; }
.opt.c2 { background-color: rgba(168, 176, 106, 0.18); color: #cdd49a; }
.opt.c3 { background-color: rgba(111, 174, 143, 0.18); color: #a3d3bb; }
.opt.c4 { background-color: rgba(182, 132, 176, 0.18); color: #d9b1d4; }
.opt.c5 { background-color: rgba(143, 147, 184, 0.18); color: #bfc2e0; }
.opt.c6 { background-color: rgba(217, 139, 149, 0.18); color: #efb5bd; }
.opt.c7 { background-color: rgba(217, 180, 74, 0.18); color: #ecd28a; }
.opt.rel { background-color: transparent; color: #e8e4dc; border-bottom: 1px solid #6f6c66; border-radius: 0; padding: 0 2px; }

/* Le choix d'une valeur (components/editeur-valeur.rsh), sous sa ligne. */
.choix-valeur { display: flex; flex-direction: column; gap: 8px; width: 300px; margin: 2px 0 8px 176px; padding: 10px; border-radius: 8px; background-color: #282725; border: 1px solid #3a3834; box-shadow: 0 8px 22px rgba(0, 0, 0, 0.3); flex-shrink: 0; }
.tr-editeur { display: flex; flex-direction: row; padding-left: 200px; }
.tr-editeur .choix-valeur { margin: 4px 0 8px 0; }
.ed-champ { box-sizing: border-box; width: 100%; height: 30px; padding: 0 8px; background-color: #1f1e1c; border: 1px solid #3e3b36; border-radius: 6px; color: #e8e4dc; font-size: 13px; }
.ed-champ:focus { border: 1px solid #c9a878; }
.ed-options { display: flex; flex-direction: row; flex-wrap: wrap; gap: 5px; }
.opt.choix { height: 24px; border: 1px solid transparent; }
.opt.choix.on { border: 1px solid #f0eeea; }
.opt.choix:hover { border: 1px solid #8c8981; }
.ed-vide { color: #8c8981; font-size: 12px; }
.ed-pied { display: flex; flex-direction: row; gap: 6px; padding-top: 6px; border-top: 1px solid #34322f; }
.ed-liste { display: flex; flex-direction: column; gap: 1px; max-height: 240px; overflow-y: auto; }
.ed-ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 28px; padding: 0 4px 0 8px; border-radius: 5px; }
.ed-ligne:hover { background-color: rgba(255, 255, 255, 0.04); }
.ed-ligne.on { background-color: rgba(201, 168, 120, 0.14); }
.ed-choix { flex-grow: 1; min-width: 0; height: 26px; padding: 0; background-color: transparent; color: #e8e4dc; font-size: 13px; text-align: left; cursor: pointer; }
.ed-ligne.on .ed-choix { color: #f0e2c8; font-weight: 600; }
.ed-aller { width: 22px; height: 22px; padding: 0; border-radius: 4px; background-color: transparent; color: #8c8981; font-size: 14px; cursor: pointer; }
.ed-aller:hover { background-color: rgba(255, 255, 255, 0.06); color: #e8e4dc; }

/* Calendrier. */
.cal-tete { display: flex; flex-direction: row; align-items: center; gap: 2px; }
.cal-titre { flex-grow: 1; padding-left: 4px; color: #eeeae2; font-size: 13px; font-weight: 600; }
.cal-nav { width: 26px; height: 26px; padding: 0; border-radius: 5px; background-color: transparent; color: #b8b5ae; font-family: monospace; font-size: 15px; cursor: pointer; }
.cal-nav:hover { background-color: rgba(255, 255, 255, 0.06); color: #f0eeea; }
.cal-ligne { display: flex; flex-direction: row; justify-content: space-between; }
.cal-sem { width: 36px; color: #6f6c66; font-size: 11px; text-align: center; }
.jour { width: 36px; height: 30px; padding: 0; border-radius: 6px; background-color: transparent; color: #e0ddd6; font-size: 12.5px; cursor: pointer; }
.jour:hover { background-color: rgba(255, 255, 255, 0.07); }
.jour.hors { color: #5d5a54; }
.jour.auj { color: #e3c99d; font-weight: 700; }
.jour.on { background-color: #c9a878; color: #1f1e1c; font-weight: 700; }

/* La carte d'une propriete (components/carte-prop.rsh). */
.prop-tete.true .prop-nom { background-color: rgba(255, 255, 255, 0.06); color: #f0eeea; }
.fiche-prop { display: flex; flex-direction: column; gap: 8px; width: 420px; margin: 2px 0 10px 0; padding: 12px; border-radius: 10px; background-color: #282725; border: 1px solid #3a3834; box-shadow: 0 10px 26px rgba(0, 0, 0, 0.32); flex-shrink: 0; }
.fiche-tete { display: flex; flex-direction: row; align-items: center; gap: 6px; }
.fiche-icone { width: 24px; color: #c9a878; font-family: monospace; font-size: 15px; text-align: center; flex-shrink: 0; }
.fiche-nom { flex-grow: 1; min-width: 0; height: 32px; padding: 0 8px; background-color: #1f1e1c; border: 1px solid #3e3b36; border-radius: 6px; color: #f0eeea; font-size: 14px; font-weight: 600; }
.fiche-nom:focus { border: 1px solid #c9a878; }
.fiche-x { width: 26px; height: 26px; padding: 0; border-radius: 5px; background-color: transparent; color: #8c8981; font-size: 14px; cursor: pointer; flex-shrink: 0; }
.fiche-x:hover { background-color: rgba(255, 255, 255, 0.06); color: #f0eeea; }
.fiche-titre { margin-top: 4px; color: #7c7972; font-size: 10px; font-weight: 700; letter-spacing: 1px; }
.fiche-types { display: flex; flex-direction: row; flex-wrap: wrap; gap: 4px; }
.fiche-type { height: 26px; padding: 0 9px; border-radius: 5px; background-color: #2e2d2a; border: 1px solid #3a3833; color: #c8c5be; font-size: 12px; cursor: pointer; }
.fiche-type:hover { background-color: #34322f; color: #f0eeea; }
.fiche-type.on { background-color: rgba(201, 168, 120, 0.16); border: 1px solid #c9a878; color: #f0e2c8; font-weight: 600; }
.fiche-options { display: flex; flex-direction: row; flex-wrap: wrap; gap: 5px; }
.fiche-option { display: flex; flex-direction: row; align-items: center; gap: 1px; }
.fiche-option .opt { cursor: default; }
.fiche-opt-x { width: 18px; height: 18px; padding: 0; border-radius: 4px; background-color: transparent; color: #6f6c66; font-size: 11px; cursor: pointer; }
.fiche-opt-x:hover { color: #e0ad94; }
.fiche-champ { box-sizing: border-box; width: 100%; height: 30px; padding: 0 8px; background-color: #1f1e1c; border: 1px solid #3e3b36; border-radius: 6px; color: #e8e4dc; font-size: 13px; }
.fiche-champ:focus { border: 1px solid #c9a878; }
.fiche-code { font-family: monospace; font-size: 12.5px; }
.fiche-aide { color: #7c7972; font-size: 11px; }
.fiche-pied { display: flex; flex-direction: row; align-items: center; gap: 6px; margin-top: 2px; padding-top: 8px; border-top: 1px solid #34322f; }
.fiche-question { flex-grow: 1; color: #e0ad94; font-size: 12px; }

/* « + Ajouter une propriété » : la liste des types. */
.types-prop { display: flex; flex-direction: column; gap: 4px; width: 420px; margin: 4px 0 10px 0; padding: 10px; border-radius: 10px; background-color: #282725; border: 1px solid #3a3834; box-shadow: 0 10px 26px rgba(0, 0, 0, 0.32); flex-shrink: 0; }
.types-liste { display: flex; flex-direction: column; gap: 1px; }
.type-ligne { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 30px; padding: 0 8px; border-radius: 6px; }
.type-ligne:hover { background-color: rgba(255, 255, 255, 0.05); }
.type-icone { width: 18px; color: #a8a59d; font-family: monospace; font-size: 14px; text-align: center; flex-shrink: 0; }
.type-nom { width: 110px; height: 28px; padding: 0; background-color: transparent; color: #eeeae2; font-size: 13px; font-weight: 600; text-align: left; cursor: pointer; flex-shrink: 0; }
.type-desc { color: #8c8981; font-size: 11.5px; }
