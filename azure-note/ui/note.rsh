<!-- Azure Note v2. Donnees : src/page.rs (arbre, page, props, blocs, et
     pour une base : vues, lignes, kanban...). Ids : voir src/clics.rs. -->
<container.app>

<container.haut>
    <container.logo><!container>
    <title3.marque>Azure Note<!title3>
    <container.chemin>
    <for.c in chemin>
        <text.chemin-sep>/<!text>
        <button.chemin-lien#p-{{c.id}}>{{c.nom}}<!button>
    <!for>
    <!container>
    <container.doc>
        <search.doc-champ#doc-q placeholder="Chercher dans la doc…"/>
        <button.doc-bouton#doc-chercher>Doc<!button>
    <!container>
<!container>

<if.a_erreur == true>
    <container.erreur><text.erreur-texte>{{erreur}}<!text><!container>
<!if>

<container.corps>

<!-- L'arbre des pages : glisser une page la range ailleurs ; ▾ replie ses
     sous-pages ; + et × au bout de chaque ligne. -->
<container.cote>
    <container.cote-tete>
        <text.cote-titre>PAGES<!text>
        <tooltip texte="Nouvelle page"><button.cote-plus#nouvelle>+<!button><!tooltip>
    <!container>
    <dropzone id="arbre"><container.liste>
    <for.n in arbre>
        <draggable id="pa-{{n.id}}"><container.item.{{n.niveau}}.{{n.etat}}.{{n.sorte}}>
            <if.n.a_enfants == true><button.item-pli#pli-{{n.id}}>{{n.pli}}<!button><!if>
            <else><container.item-pli-vide><!container><!else>
            <container.puce-page.t-{{n.teinte}}><!container>
            <button.item-nom#p-{{n.id}}>{{n.nom}}<!button>
            <if.n.base == false><tooltip texte="Nouvelle sous-page"><button.item-act#sous-{{n.id}}>+<!button><!tooltip><!if>
            <tooltip texte="Supprimer"><button.item-act.item-x#arbre-suppr-{{n.id}}>×<!button><!tooltip>
        <!container><!draggable>
        <if.n.confirmer == true>
        <container.item-confirmer>
            <text.item-question>Supprimer « {{n.nom}} » et ses sous-pages ?<!text>
            <container.item-choix>
                <button.btn#arbre-non>Annuler<!button>
                <button.btn.danger#arbre-oui-{{n.id}}>Supprimer<!button>
            <!container>
        <!container>
        <!if>
    <!for>
    <!container><!dropzone>
    <container.cote-bas>
        <button.cote-lien#nouvelle>+  Nouvelle page<!button>
        <button.cote-lien#nouvelle-base>+  Nouvelle base<!button>
    <!container>
<!container>

<container.editeur>
<if.a_page == true>
<dropzone id="dans-{{page.id}}"><container.feuille>

    <!-- L'identite de la page : sa couverture et sa pastille, de sa couleur. -->
    <container.couverture.t-{{page.teinte}}><!container>
    <container.col-page.{{page.largeur}}>
    <container.entete-page>
        <tooltip texte="Changer la couleur"><button.pastille.t-{{page.teinte}}#pastille>{{page.initiale}}<!button><!tooltip>
        <button.lien-discret.options-bouton#options>Options<!button>
    <!container>

    <richtext.titre#titre valeur="{{page.titre}}" placeholder="Sans titre" entree="true" focus="{{focus_titre}}"><!richtext>
    <!-- Options de la page (discretes) : couleur, proprietes, suppression. -->
    <if.options == true>
    <container.options>
        <container.options-ligne>
            <text.options-nom>Couleur<!text>
            <for.c in teintes><button.teinte.t-{{c.nom}}.{{c.on}}#teinte-{{c.nom}}><!button><!for>
        <!container>
        <container.options-ligne>
            <text.options-nom>Largeur<!text>
            <for.l in largeurs><button.choix-largeur.{{l.on}}#largeur-{{l.code}}>{{l.nom}}<!button><!for>
        <!container>
        <container.options-ligne>
            <text.options-nom>Modifiée<!text>
            <text.meta-info>{{page.depuis}}<!text>
        <!container>
        <container.options-ligne>
            <if.page.base == false><button.lien-discret#prop-ouvrir>+ Propriété<!button><!if>
            <if.confirmer == true>
                <text.meta-question>Supprimer « {{page.nom}} » et ses sous-pages ?<!text>
                <button.btn#supprimer-non>Annuler<!button>
                <button.btn.danger#supprimer-oui>Supprimer<!button>
            <!if>
            <else>
                <button.lien-discret.rouge#supprimer>Supprimer la page<!button>
            <!else>
        <!container>
    <!container>
    <!if>

    <!-- Proprietes de la page : un clic sur le nom ouvre sa carte (nom,
         type, options, formule) ; un clic sur une valeur ouvre son choix
         (options, calendrier, pages). Voir src/proprietes.rs. -->
    <if.a_props == true>
    <container.props>
        <for.p in props>
            <container.prop>
                <container.prop-tete.{{p.menu}}>
                    <text.prop-icone>{{p.icone}}<!text>
                    <button.prop-nom#prop-menu-{{p.cle}}>{{p.nom}}<!button>
                <!container>
                <container.prop-val><champ/><!container>
            <!container>
            <if.p.ouvert == true><editeur-valeur/><!if>
            <if.p.menu == true><carte-prop/><!if>
        <!for>
        <if.ajout_propriete == true><types-prop/><!if>
        <else><button.prop-ajout#prop-ouvrir>+  Ajouter une propriété<!button><!else>
    <!container>
    <!if>

    <!-- Une base : ses vues. -->
    <if.page.base == true>
    <container.base>
        <container.onglets>
            <for.v in vues>
                <if.v.actif == true><button.onglet.on#vue-{{v.n}}>{{v.nom}}<!button><!if>
                <else><button.onglet#vue-{{v.n}}>{{v.nom}}<!button><!else>
            <!for>
            <container.onglets-fin>
                <button.lien-discret#vue-ajout-table>+ Table<!button>
                <button.lien-discret#vue-ajout-kanban>+ Kanban<!button>
                <button.lien-discret#vue-ajout-liste>+ Liste<!button>
                <button.lien-discret#vue-ajout-galerie>+ Galerie<!button>
                <button.lien-discret#filtres>Filtres et tri<!button>
                <button.lien-discret#vue-suppr>Retirer la vue<!button>
            <!container>
        <!container>

        <if.a_carte == true><carte-prop/><!if>
        <if.ajout_propriete == true><types-prop/><!if>

        <if.filtres_ouverts == true>
        <container.filtres>
            <container.filtres-ligne>
                <text.filtres-titre>Filtrer<!text>
                <select.f-prop#filtre-prop options="{{props_base}}"/>
                <select.f-op#filtre-op options="{{operateurs}}"/>
                <input.f-val#filtre-val placeholder="valeur"/>
                <button.btn#filtre-ajouter>Ajouter<!button>
            <!container>
            <container.filtres-ligne>
                <text.filtres-titre>Trier<!text>
                <select.f-prop#tri options="{{props_base}}" value="{{tri}}"/>
                <select.f-op#tri-sens options="Croissant, Décroissant" value="{{tri_sens}}"/>
                <if.affichage == "kanban">
                    <text.filtres-titre>Colonnes<!text>
                    <select.f-prop#groupe options="{{groupables}}" value="{{groupe}}"/>
                <!if>
            <!container>
            <container.filtres-ligne>
            <for.f in filtres>
                <container.puce-filtre><text.puce-texte>{{f.texte}}<!text><button.puce-x#filtre-suppr-{{f.i}}>×<!button><!container>
            <!for>
            <!container>
        <!container>
        <!if>

        <if.affichage == "kanban">
        <container.kanban>
            <for.k in kanban>
                <dropzone id="col-{{k.i}}"><container.colonne>
                    <container.col-tete><text.col-nom>{{k.nom}}<!text><text.col-nb>{{k.nb}}<!text><!container>
                    <for.c in k.cartes>
                        <draggable id="k-{{c.id}}-{{k.i}}"><container.carte>
                            <button.carte-nom#p-{{c.id}}>{{c.nom}}<!button>
                            <for.d in c.details><text.carte-detail>{{d.nom}} · {{d.v}}<!text><!for>
                        <!container><!draggable>
                    <!for>
                    <button.col-plus#ligne-col-{{k.i}}>+ Nouvelle<!button>
                <!container><!dropzone>
            <!for>
        <!container>
        <!if>

        <elseif.affichage == "galerie">
        <container.galerie>
            <for.l in lignes>
                <container.vignette>
                    <button.carte-nom#p-{{l.id}}>{{l.nom}}<!button>
                    <for.c in l.cellules><if.c.texte != ""><text.carte-detail>{{c.nom}} · {{c.texte}}<!text><!if><!for>
                <!container>
            <!for>
            <button.vignette-plus#ligne-ajouter>+ Nouvelle<!button>
        <!container>
        <!elseif>

        <elseif.affichage == "liste">
        <dropzone id="lignes"><container.liste-base>
            <for.l in lignes>
                <draggable id="li-{{l.id}}"><container.ligne-liste>
                    <button.ligne-nom#p-{{l.id}}>{{l.nom}}<!button>
                    <for.c in l.cellules><if.c.texte != ""><text.ligne-detail>{{c.texte}}<!text><!if><!for>
                <!container><!draggable>
            <!for>
        <!container><!dropzone>
        <button.lien-discret#ligne-ajouter>+ Nouvelle ligne<!button>
        <!elseif>

        <else>
        <container.table>
            <container.table-tete>
                <text.th.th-nom>Nom<!text>
                <for.c in colonnes>
                    <container.th.{{c.on}}><text.th-icone>{{c.icone}}<!text><button.th-texte#prop-menu-{{c.cle}}>{{c.nom}}<!button><!container>
                <!for>
                <tooltip texte="Ajouter une propriété"><button.th-plus#prop-ouvrir>+<!button><!tooltip>
            <!container>
            <dropzone id="lignes"><container.table-corps>
            <for.l in lignes>
                <draggable id="li-{{l.id}}"><container.tr>
                    <container.td.td-nom><button.ligne-nom#p-{{l.id}}>{{l.nom}}<!button><!container>
                    <for.p in l.cellules>
                        <container.td><champ/><!container>
                    <!for>
                <!container><!draggable>
                <if.l.ouvert == true><container.tr-editeur><editeur-valeur/><!container><!if>
            <!for>
            <!container><!dropzone>
            <button.lien-discret#ligne-ajouter>+ Nouvelle ligne<!button>
        <!container>
        <!else>
    <!container>
    <!if>

    <!-- Les blocs : glisser la poignee pour les ranger. -->
    <dropzone id="blocs"><container.blocs>
    <for.b in blocs>
        <draggable id="bl-{{b.id}}"><container.bloc.{{b.genre}}>
            <if.b.genre != "separateur"><tooltip texte="Glisser pour déplacer"><text.poignee>::<!text><!tooltip><!if>
            <if.b.genre == "puce"><text.marque-liste>•<!text><!if>
            <elseif.b.genre == "numero"><text.marque-liste>{{b.numero}}.<!text><!elseif>
            <elseif.b.genre == "tache"><checkbox.tache#t-{{b.id}} checked="{{b.fait}}"/><!elseif>

            <if.b.texte_riche == true>
                <richtext.bloc-texte.{{b.genre}}#b-{{b.id}} valeur="{{b.valeur}}" placeholder="Tape / pour insérer un bloc" commandes="{{commandes}}" entree="true" focus="{{b.focus}}"><!richtext>
            <!if>
            <elseif.b.genre == "code">
                <container.code-bloc>
                    <input.code-langage#lang-{{b.id}} value="{{b.langage}}" placeholder="langage"/>
                    <richtext.code-texte#b-{{b.id}} valeur="{{b.valeur}}" placeholder="// du code" focus="{{b.focus}}"><!richtext>
                <!container>
            <!elseif>
            <elseif.b.genre == "tableau">
                <container.tableau>
                <for.l in b.lignes>
                    <container.tab-ligne>
                    <for.c in l.cellules><input.{{c.cl}}#c-{{b.id}}-{{l.r}}-{{c.c}} value="{{c.v}}"/><!for>
                    <!container>
                <!for>
                <container.tab-outils>
                    <button.lien-discret#bloc-ligne-{{b.id}}>+ Ligne<!button>
                    <button.lien-discret#bloc-col-{{b.id}}>+ Colonne<!button>
                <!container>
                <!container>
            <!elseif>
            <elseif.b.genre == "separateur"><container.trait><!container><!elseif>
            <elseif.b.genre == "souspage"><container.lien-ligne><container.puce-page.t-{{b.teinte}}><!container><button.lien-page#p-{{b.cible}}>{{b.nom}}<!button><!container><!elseif>
            <elseif.b.genre == "vue">
                <container.vue-bloc>
                    <button.lien-page#p-{{b.cible}}>Base · {{b.nom}}<!button>
                    <container.mini-table>
                        <container.mini-tr.mini-tete><text.mini-td>Nom<!text><for.c in b.colonnes><text.mini-td>{{c.nom}}<!text><!for><!container>
                        <for.l in b.lignes>
                            <container.mini-tr><button.mini-nom#p-{{l.id}}>{{l.nom}}<!button><for.c in l.cellules><text.mini-td>{{c.texte}}<!text><!for><!container>
                        <!for>
                    <!container>
                <!container>
            <!elseif>
            <if.b.genre != "separateur"><tooltip texte="Supprimer le bloc"><button.bloc-x#bloc-suppr-{{b.id}}>×<!button><!tooltip><!if>
        <!container><!draggable>
        <if.b.menu == true><menu-blocs/><!if>
    <!for>
    <!container><!dropzone>

    <if.page.base == false>
    <if.menu_fin == true><menu-blocs/><!if>
    <!-- Le reste de la page : un clic ici pour ecrire a la fin. -->
    <button.zone-ecrire#zone-ecrire><!button>

    <if.a_sous_pages == true>
    <container.sous-pages>
    <for.s in sous_pages>
        <container.lien-ligne><container.puce-page.t-{{s.teinte}}><!container><button.lien-page#p-{{s.id}}>{{s.nom}}<!button><!container>
    <!for>
    <!container>
    <!if>
    <!if>

    <!container>
<!container><!dropzone>
<!if>
<else>
    <container.vide>
        <title2.vide-titre>Aucune page<!title2>
        <text.vide-texte>Des pages dans des pages, du texte mis en forme, du code, des tableaux, et des bases avec vues table, kanban, liste ou galerie.<!text>
        <text.vide-texte>Une propriété peut se calculer depuis les sous-pages : moyenne(enfants.avancement).<!text>
        <button.btn.primaire#nouvelle>Créer une page<!button>
    <!container>
<!else>
<!container>

<!container>
<!container>
