<!-- Onglet Tests : ce que l'analyse a trouve, par fichier ; le test choisi
     a droite, avec son code dans un bloc facon Azure Note. -->
<container.tests>
    <container.entete>
        <container.entete-texte>
            <title2.h>{{projet.nom}}<!title2>
            <text.sous>{{projet.chemin}} · {{projet.langage}} · {{projet.nb_paquets}} paquet(s)<!text>
        <!container>
        <button.bouton#analyser>Analyser<!button>
        <button.bouton#lancer-visibles>Lancer l'affichage<!button>
        <if.nb_echoues \> 0><button.bouton#relancer-echecs>Relancer les échecs<!button><!if>
        <button.bouton.fort#tout-lancer>Tout lancer<!button>
    <!container>

    <container.chiffres>
        <container.chiffre><text.chiffre-n>{{nb_tests}}<!text><text.chiffre-l>tests<!text><!container>
        <container.chiffre.ok><text.chiffre-n>{{nb_reussis}}<!text><text.chiffre-l>réussis<!text><!container>
        <container.chiffre.ko><text.chiffre-n>{{nb_echoues}}<!text><text.chiffre-l>échoués<!text><!container>
        <container.chiffre><text.chiffre-n>{{nb_jamais}}<!text><text.chiffre-l>pas lancés<!text><!container>
        <container.chiffre><text.chiffre-n>{{nb_ignores}}<!text><text.chiffre-l>ignorés<!text><!container>
    <!container>

    <container.outils>
        <input.filtre#filtre placeholder="Filtrer : nom, module ou fichier (Entrée)" value="{{saisie_filtre}}"/>
        <if.filtre != ""><button.petit#effacer-filtre>Effacer<!button><!if>
        <container.vues>
        <for.v in vues>
            <if.v.on == true><button.vue.on#vue-{{v.code}}>{{v.nom}}<!button><!if>
            <else><button.vue#vue-{{v.code}}>{{v.nom}}<!button><!else>
        <!for>
        <!container>
        <button.petit#ouvrir-tout>Tout déplier<!button>
        <button.petit#fermer-tout>Tout replier<!button>
    <!container>

    <container.zone>
        <container.liste>
            <if.nb_tests == 0><text.vide-texte>Aucun test trouvé dans ce projet. Créez-en un dans l'Atelier.<!text><!if>
            <elseif.nb_visibles == 0><text.vide-texte>Aucun test ne correspond.<!text><!elseif>
            <for.g in groupes>
                <container.groupe>
                    <container.groupe-tete>
                        <if.g.ouvert == true><text.pli>–<!text><!if>
                        <else><text.pli>+<!text><!else>
                        <button.groupe-nom#groupe-{{g.premier}}>{{g.nom}}<!button>
                        <text.groupe-dossier>{{g.dossier}}<!text>
                        <text.groupe-nb>{{g.nb}}<!text>
                        <if.g.nb_echecs \> 0><text.pastille.ko>{{g.nb_echecs}} échec(s)<!text><!if>
                        <elseif.g.nb_reussis == g.nb><text.pastille.ok>tout passe<!text><!elseif>
                        <button.mini#lancer-groupe-{{g.premier}}>Lancer<!button>
                    <!container>
                    <if.g.coupe == true><text.coupe>Lignes non affichées : filtrez ou repliez d'autres fichiers.<!text><!if>
                    <for.x in g.tests>
                        <container.ligne>
                            <container.point.{{x.statut}}><!container>
                            <if.x.choisi == true><button.nom-test.on#voir-{{x.i}}>{{x.nom}}<!button><!if>
                            <else><button.nom-test#voir-{{x.i}}>{{x.nom}}<!button><!else>
                            <if.x.ignore == true><text.pastille>ignoré par défaut<!text><!if>
                            <text.etat.{{x.statut}}>{{x.libelle}}<!text>
                            <button.mini#lancer-{{x.i}}>Lancer<!button>
                            <button.mini.discret#ecrire-{{x.i}}>Modifier<!button>
                        <!container>
                    <!for>
                <!container>
            <!for>
            <if.nb_caches \> 0><text.coupe>{{nb_caches}} test(s) non affichés pour garder l'app légère : filtrez (nom, fichier) ou repliez des fichiers.<!text><!if>
            <for.a in avertissements><text.avert>{{a}}<!text><!for>
        <!container>

        <if.a_detail == true>
            <container.detail>
                <container.detail-tete>
                    <title3.detail-nom>{{detail.nom}}<!title3>
                    <button.mini.discret#fermer-detail>×<!button>
                <!container>
                <text.sous>{{detail.chemin}}<!text>
                <text.sous>{{detail.fichier}} · ligne {{detail.ligne}}<!text>
                <container.detail-etat>
                    <container.point.{{detail.statut}}><!container>
                    <text.etat.{{detail.statut}}>{{detail.libelle}}<!text>
                <!container>
                <container.rangee>
                    <button.bouton.fort#d-lancer-{{detail.i}}>Lancer<!button>
                    <button.bouton#d-ecrire-{{detail.i}}>Modifier dans l'atelier<!button>
                <!container>
                <if.detail.a_sortie == true>
                    <text.etiquette>Sortie de l'échec<!text>
                    <container.sortie>
                        <for.l in detail.sortie><text.sortie-ligne>{{l}}<!text><!for>
                    <!container>
                <!if>
                <text.etiquette>Code<!text>
                <container.code-bloc>
                    <richtext.code-texte#detail-code valeur="{{detail.code}}"><!richtext>
                <!container>
            <!container>
        <!if>
    <!container>
<!container>
