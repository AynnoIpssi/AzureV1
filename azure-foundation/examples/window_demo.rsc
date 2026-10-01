/* Feuille de style rsC pour la demonstration visuelle (run_window_demo) :
   chaque type d'element .rsh recoit une position/couleur distincte, pour
   que "chaque chose" reste identifiable a l'oeil dans la fenetre. `top`/
   `left` sont les deux seules proprietes de positionnement deja reliees au
   moteur de rendu (voir services::link::to_legacy_style) - pas encore
   `position`/`right`/`bottom`, ni un vrai flux (chaque pourcentage est
   toujours relatif a la fenetre entiere, pas au parent). */

container {
    background-color: #7c7cc0;
}

.hero {
    width: 100%;
    height: 100%;
    background-color: #321e21;
}

/* Selecteurs de type : chaque niveau de titre a sa propre position, sans
   avoir besoin de classe. */
title1 {
    top: 4%;
    left: 5%;
    color: #ffffff;
    font-weight: 700;
}

title2 {
    top: 11%;
    left: 5%;
    color: #a0a0b0;
}

title3 {
    top: 16%;
    left: 5%;
    color: #a0a0b0;
}

text {
    top: 20%;
    left: 5%;
    color: #ffffff;
}

.card {
    top: 30%;
    left: 5%;
    width: 90%;
    height: 55%;
    background-color: #2a2a45;
    border-radius: 12px;
}

.primary-button {
    width: 34%;
    height: 8%;
    background-color: #3b82f6;
    color: #ffffff;
}

/* Selecteurs d'id : les deux boutons partagent `.primary-button` (taille,
   couleur) mais chacun a sa propre position - demontre la cascade
   classe -> id (l'id, plus specifique, ne change ici que top/left). */
#btn-users {
    top: 34%;
    left: 8%;
}

#btn-logs {
    top: 34%;
    left: 46%;
}

image {
    top: 46%;
    left: 8%;
    width: 34%;
    height: 12%;
}

video {
    top: 46%;
    left: 46%;
    width: 34%;
    height: 12%;
}

/* Interactive : cliquer dedans la focalise (contour blanc), puis taper au
   clavier modifie son contenu - voir ui::services::interact. */
textarea {
    top: 62%;
    left: 8%;
    width: 72%;
    height: 12%;
    background-color: #0f0f19;
    color: #ffffff;
}
