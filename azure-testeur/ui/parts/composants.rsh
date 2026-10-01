<!-- Composants : la page du stockage, du dossier ou du composant choisi
     dans la barre de gauche. Donnees : src/ecran.rs (`vue_composants`). -->
<container.feuille>
<if.comp_a_choix == false>
    <container.vide>
        <title2.vide-titre>Composants<!title2>
        <text.vide-texte>Vos morceaux de code de test, rangés dans des stockages et des dossiers (autant de niveaux que vous voulez). Dans l'Atelier, « Nouveau test » les ajoute au test.<!text>
        <container.rangee><button.bouton.fort#d-comp-nouveau-stockage>+ Créer un stockage<!button><!container>
    <!container>
<!if>
<else>
    <container.col-page>
        <container.couverture.{{comp.genre}}><!container>
        <container.fil>
            <for.c in comp.chemin>
                <button.fil-lien#d-noeud-{{c.id}}>{{c.nom}}<!button>
                <if.c.dernier == false><text.fil-sep>›<!text><!if>
            <!for>
        <!container>
        <container.rangee>
            <text.genre-pastille.{{comp.genre}}>{{comp.libelle}}<!text>
            <if.comp.genre == "composant"><text.sous>{{comp.lignes}} ligne(s)<!text><!if>
            <else><text.sous>{{comp.nb_dedans}} élément(s) dedans<!text><!else>
        <!container>
        <container.rangee>
            <input.titre-champ#comp-nom value="{{comp.nom}}" focus="{{comp.nommer}}"/>
            <button.bouton#comp-renommer>Renommer<!button>
        <!container>
        <container.rangee>
            <if.comp.contenant == true>
                <button.bouton#comp-dossier>+ Dossier<!button>
                <button.bouton#comp-composant>+ Composant<!button>
            <!if>
            <container.pousse><!container>
            <if.comp.confirmer == true>
                <text.sous>Supprimer « {{comp.nom}} » et ses {{comp.nb_dedans}} élément(s) ?<!text>
                <button.bouton.danger#comp-supprimer-oui>Supprimer<!button>
                <button.bouton#comp-supprimer-non>Garder<!button>
            <!if>
            <else><button.mini.discret#comp-supprimer>Supprimer<!button><!else>
        <!container>

        <if.comp.genre == "composant">
            <text.section>Code<!text>
            <container.code-bloc>
                <richtext.code-texte#comp-code valeur="{{comp.code}}" placeholder="// le code ajouté au test (ex. let x = 2; assert_eq!(x * 2, 4);)"><!richtext>
            <!container>
            <container.rangee>
                <button.bouton.fort#comp-enregistrer>Enregistrer<!button>
                <if.comp.modifie == true>
                    <text.pastille.attente>pas encore enregistré<!text>
                    <button.mini.discret#comp-annuler>Annuler<!button>
                <!if>
            <!container>
        <!if>
        <else>
            <text.section>Contenu<!text>
            <if.comp.a_enfants == false><text.vide-texte>Vide : ajoutez un dossier ou un composant.<!text><!if>
            <for.c in comp.enfants>
                <container.ligne-bloc>
                    <container.arb-ic.{{c.genre}}><!container>
                    <button.ligne-titre#d-noeud-{{c.id}}>{{c.nom}}<!button>
                    <if.c.genre != "composant"><text.ligne-no>{{c.nb}} élément(s)<!text><!if>
                    <text.ligne-no>{{c.libelle}}<!text>
                <!container>
            <!for>
        <!else>

        <if.comp.a_destinations == true>
            <text.section>Ranger ailleurs<!text>
            <container.rangee>
                <select.champ.large#comp-destination options="{{comp.destinations}}"/>
                <button.bouton#comp-deplacer>Déplacer<!button>
            <!container>
        <!if>
    <!container>
<!else>
<!container>
