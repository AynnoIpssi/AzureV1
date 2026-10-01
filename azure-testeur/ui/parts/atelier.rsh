<!-- Atelier : une page facon Azure Note par fichier de tests. Son code
     generique, puis chaque test dans un bloc de code ; « Écrire dans le
     code » modifie directement le fichier du projet. -->
<container.atelier>
    <container.fichiers>
        <text.cote-titre>FICHIERS DE TESTS<!text>
        <container.fichiers-liste>
        <for.f in fichiers>
            <if.f.actif == true><button.fichier.on#fichier-{{f.k}}>{{f.chemin}} · {{f.nb}}<!button><!if>
            <else><button.fichier#fichier-{{f.k}}>{{f.chemin}} · {{f.nb}}<!button><!else>
        <!for>
        <!container>
        <button.ajout#nouveau-fichier>+ Nouveau fichier de tests<!button>
        <if.nouveau_fichier == true>
            <container.formulaire>
                <text.etiquette>Paquet<!text>
                <select.champ#nf-paquet options="{{paquets}}"/>
                <text.etiquette>Nom du fichier<!text>
                <input.champ#nf-nom placeholder="calculs" value="{{nf_nom}}"/>
                <text.cote-aide>Créé dans le dossier tests/ du paquet.<!text>
                <button.bouton.fort#nf-creer>Créer le fichier<!button>
            <!container>
        <!if>
    <!container>

    <container.feuille>
    <if.a_fichier == false>
        <container.vide>
            <title2.vide-titre>Atelier<!title2>
            <text.vide-texte>Choisissez un fichier de tests à gauche, ou créez-en un : les tests s'écrivent directement dans le code du projet.<!text>
        <!container>
    <!if>
    <else>
        <container.col-page>
            <container.couverture><!container>
            <title1.page-titre>{{atelier.chemin}}<!title1>
            <container.rangee>
                <text.sous>{{atelier.nb}} test(s) · écrits dans le code du projet<!text>
                <button.bouton#lancer-fichier>Lancer le fichier<!button>
            <!container>

            <!-- Liste legere : une ligne par bloc. Un seul s'ouvre en
                 editeur de code (clic), le reste ne coute rien a dessiner. -->
            <text.section>Code générique<!text>
            <if.atelier.generique_ouvert == true>
                <container.bloc-test.ouvert>
                    <container.rangee>
                        <button.ligne-titre.on#ouvrir-generique>Code générique<!button>
                        <text.aide>Partagé par tous les tests du fichier : imports, fonctions d'aide.<!text>
                    <!container>
                    <container.code-bloc>
                        <richtext.code-texte#generique valeur="{{atelier.generique}}" placeholder="// use, fonctions d'aide..."><!richtext>
                    <!container>
                    <container.rangee>
                        <button.bouton.fort#generique-enregistrer>Écrire dans le code<!button>
                        <if.atelier.generique_modifie == true><button.mini.discret#generique-annuler>Annuler<!button><!if>
                        <container.pousse><!container>
                        <button.mini.discret#fermer-editeur>Fermer<!button>
                    <!container>
                <!container>
            <!if>
            <else>
                <container.ligne-bloc>
                    <button.ligne-titre#ouvrir-generique>Code générique<!button>
                    <if.atelier.generique_modifie == true><text.pastille.attente>pas encore écrit<!text><!if>
                    <text.ligne-no>{{atelier.generique_lignes}} ligne(s)<!text>
                <!container>
            <!else>

            <text.section>Tests · {{atelier.nb}}<!text>
            <for.x in atelier.tests>
                <if.x.ouvert == true>
                    <container.bloc-test.ouvert#at-{{x.i}}>
                        <container.rangee>
                            <input.nom-champ#nom-{{x.i}} value="{{x.nom}}"/>
                            <container.point.{{x.statut}}><!container>
                            <text.etat.{{x.statut}}>{{x.libelle}}<!text>
                            <if.x.modifie == true><text.pastille.attente>pas encore écrit<!text><!if>
                            <text.ligne-no>ligne {{x.ligne}}<!text>
                        <!container>
                        <container.code-bloc>
                            <richtext.code-texte#code-{{x.i}} valeur="{{x.code}}"><!richtext>
                        <!container>
                        <container.options>
                        <for.o in x.options>
                            <checkbox.option#opt-{{x.i}}-{{o.code}} checked="{{o.coche}}">{{o.libelle}}<!checkbox>
                        <!for>
                        <!container>
                        <if.x.a_sortie == true>
                            <container.sortie>
                                <for.l in x.sortie><text.sortie-ligne>{{l}}<!text><!for>
                            <!container>
                        <!if>
                        <container.rangee>
                            <button.bouton.fort#enregistrer-{{x.i}}>Écrire dans le code<!button>
                            <button.bouton#lancer-at-{{x.i}}>Lancer<!button>
                            <if.x.modifie == true><button.mini.discret#annuler-{{x.i}}>Annuler<!button><!if>
                            <container.pousse><!container>
                            <if.x.confirmer == true>
                                <text.sous>Retirer ce test du code ?<!text>
                                <button.bouton.danger#supprimer-oui-{{x.i}}>Supprimer<!button>
                                <button.bouton#supprimer-non>Garder<!button>
                            <!if>
                            <else>
                                <button.mini.discret#supprimer-{{x.i}}>Supprimer<!button>
                                <button.mini.discret#fermer-editeur>Fermer<!button>
                            <!else>
                        <!container>
                    <!container>
                <!if>
                <else>
                    <container.ligne-bloc#at-{{x.i}}>
                        <container.point.{{x.statut}}><!container>
                        <button.ligne-titre#ouvrir-{{x.i}}>{{x.nom}}<!button>
                        <if.x.modifie == true><text.pastille.attente>pas encore écrit<!text><!if>
                        <text.etat.{{x.statut}}>{{x.libelle}}<!text>
                        <text.ligne-no>ligne {{x.ligne}}<!text>
                        <button.mini#lancer-at-{{x.i}}>Lancer<!button>
                    <!container>
                <!else>
            <!for>

            <if.atelier.nouveau_ouvert == true>
                <text.section>Nouveau test<!text>
                <container.bloc-test.nouveau>
                    <input.nom-champ#nouveau-nom placeholder="nom_du_test (lettres, chiffres, _)" value="{{atelier.nouveau_nom}}" focus="true"/>
                    <container.code-bloc>
                        <richtext.code-texte#nouveau-code valeur="{{atelier.nouveau_code}}" placeholder="// le corps du test"><!richtext>
                    <!container>
                    <!-- Vos composants (barre de gauche) : leur code s'ajoute au test. -->
                    <container.rangee>
                        <if.a_composants == true>
                            <select.champ.large#composant-choix options="{{composants_options}}"/>
                            <button.bouton#inserer-composant>Ajouter le composant<!button>
                        <!if>
                        <else><text.aide>Pas encore de composant : créez-en dans la barre de gauche (COMPOSANTS).<!text><!else>
                    <!container>
                    <container.options>
                    <for.o in atelier.nouveau_options>
                        <checkbox.option#nouveau-opt-{{o.code}} checked="{{o.coche}}">{{o.libelle}}<!checkbox>
                    <!for>
                    <!container>
                    <container.rangee>
                        <button.bouton.fort#creer-test>Créer le test dans le projet<!button>
                        <container.pousse><!container>
                        <button.mini.discret#fermer-editeur>Fermer<!button>
                    <!container>
                <!container>
            <!if>
            <else><button.ajout#ouvrir-nouveau>+ Nouveau test<!button><!else>
        <!container>
    <!else>
    <!container>
<!container>
