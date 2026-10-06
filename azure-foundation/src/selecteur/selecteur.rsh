<!-- La boite « Ouvrir » d'Azure (voir `selecteur/mod.rs`) : les lieux a
     gauche, le dossier ouvert a droite, le choix en bas. Les ids `sel-…`
     sont lus par `Ouvert::cliquer`. -->
<container.sel-fond>
    <container.sel-boite>
        <container.sel-tete>
            <text.sel-titre>{{titre}}<!text>
            <button.sel-x#selecteur-fermer>x<!button>
        <!container>

        <container.sel-barre>
            <button.sel-outil#sel-parent>Dossier parent<!button>
            <container.sel-fil>
                <for.f in fil>
                    <if.f.dernier == true><text.sel-fil-ici>{{f.nom}}<!text><!if>
                    <else>
                        <button.sel-fil-b#sel-fil-{{f.i}}>{{f.nom}}<!button>
                        <text.sel-fil-sep>/<!text>
                    <!else>
                <!for>
            <!container>
            <if.caches == true><button.sel-outil.on#sel-caches>Fichiers cachés<!button><!if>
            <else><button.sel-outil#sel-caches>Fichiers cachés<!button><!else>
        <!container>

        <container.sel-corps>
            <container.sel-lieux>
                <text.sel-petit>LIEUX<!text>
                <for.l in lieux>
                    <if.l.ici == true><button.sel-lieu.ici#sel-lieu-{{l.i}}>{{l.nom}}<!button><!if>
                    <else><button.sel-lieu#sel-lieu-{{l.i}}>{{l.nom}}<!button><!else>
                <!for>
                <if.a_lieux == false><text.sel-aide>Aucun dossier autorisé.<!text><!if>
            <!container>

            <container.sel-liste>
                <if.a_erreur == true><text.sel-erreur>{{erreur}}<!text><!if>
                <elseif.vide == true><text.sel-aide>{{vide_texte}}<!text><!elseif>
                <for.e in entrees>
                    <container.sel-ligne>
                        <container.sel-ic.{{e.genre}}><!container>
                        <if.e.etat == "inactif"><text.sel-nom.inactif>{{e.nom}}<!text><!if>
                        <elseif.e.etat == "choisi"><button.sel-nom.choisi#sel-entree-{{e.i}}>{{e.nom}}<!button><!elseif>
                        <else><button.sel-nom#sel-entree-{{e.i}}>{{e.nom}}<!button><!else>
                    <!container>
                <!for>
                <if.tronque == true><text.sel-aide>{{reste}}<!text><!if>
            <!container>
        <!container>

        <container.sel-pied>
            <text.sel-choix>{{choix}}<!text>
            <button.sel-bouton#sel-annuler>Annuler<!button>
            <if.pret == true><button.sel-bouton.fort#sel-choisir>{{action}}<!button><!if>
            <else><text.sel-bouton.eteint>{{action}}<!text><!else>
        <!container>
    <!container>
<!container>
