/* Feuille de style rsC - vraie syntaxe CSS (selecteurs de balise, de
   classe, d'id, combinateurs, cascade par specificite/!important),
   appliquee directement sur l'arbre demo.rsh via le systeme de lien
   (voir azure_foundation::rsc::services::link et
   compiler::services::codegen::generate_with_rsc). */

container {
    background-color: #14141f;
}

.hero {
    width: 90%;
    height: 40%;
    padding: 8px;
    background-color: #1e1e32;
    color: #ffffff;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.card {
    width: 90%;
    height: 95%;
    padding: 4px;
    background-color: #2a2a45;
    border-radius: 12px;
}

/* Selecteur descendant : seuls les boutons a l'interieur d'un .card
   recoivent ce style - un bouton place ailleurs dans l'arbre n'est pas
   concerne (voir services::link::complex_matches). */
.card button {
    background-color: #3b82f6;
    color: white;
}

.primary-button {
    width: 30%;
    height: 20%;
    background-color: #3b82f6;
    color: #ffffff;
}

.muted-text {
    color: rgba(160, 160, 176, 0.8);
    font-size: 14px;
}

title1 {
    font-weight: 700;
}

/* Pseudo-classe reconnue syntaxiquement mais pas encore evaluee (voir
   SimpleSelector::pseudo_classes) : cette regle ne correspond a aucun
   element pour l'instant, elle ne fait pas echouer le parsing pour autant. */
button:hover {
    background-color: #2563eb;
}
