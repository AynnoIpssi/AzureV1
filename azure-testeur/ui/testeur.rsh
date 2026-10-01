<!-- Azure Testeur. Donnees : voir src/ecran.rs (`contexte`). Deux onglets :
     les tests du projet ouvert (parts/tests.rsh) et l'atelier, ou l'on
     ecrit des tests directement dans le code (parts/atelier.rsh). -->
<container.app>

<container.haut>
    <container.logo><!container>
    <text.marque>Azure Testeur<!text>
    <if.a_projet == true>
        <if.onglet == "tests"><button.onglet.on#onglet-tests>Tests<!button><!if>
        <else><button.onglet#onglet-tests>Tests<!button><!else>
        <if.onglet == "atelier"><button.onglet.on#onglet-atelier>Atelier<!button><!if>
        <else><button.onglet#onglet-atelier>Atelier<!button><!else>
    <!if>
    <if.onglet == "composants"><button.onglet.on#onglet-composants>Composants<!button><!if>
    <else><button.onglet#onglet-composants>Composants<!button><!else>
    <container.etat-exec>
        <if.en_cours == true>
            <container.point-vivant><!container>
            <text.exec>{{exec_titre}} {{exec_etape}}<!text>
            <button.petit.danger#arreter>Arrêter<!button>
        <!if>
        <elseif.bilan != "">
            <if.echec_commande == true><text.exec.ko>{{bilan}}<!text><!if>
            <else><text.exec>{{bilan}}<!text><!else>
        <!elseif>
        <if.a_sortie == true>
            <if.console == true><button.petit.on#console>Sortie<!button><!if>
            <else><button.petit#console>Sortie<!button><!else>
        <!if>
    <!container>
<!container>

<!-- Toujours un seul element ici (bandeau ou vide) : l'ecran garde la meme
     forme, donc son defilement, quand un message apparait. -->
<if.erreur != "">
    <container.alerte.ko><text.alerte-texte>{{erreur}}<!text><button.alerte-x#fermer-message>Fermer<!button><!container>
<!if>
<elseif.message != "">
    <container.alerte.ok><text.alerte-texte>{{message}}<!text><button.alerte-x#fermer-message>Fermer<!button><!container>
<!elseif>
<else><container.sans-alerte><!container><!else>

<container.corps>
    <!-- Projets relies : l'environnement Azure, puis ceux de l'utilisateur. -->
    <container.cote>
        <text.cote-titre>PROJETS<!text>
        <container.projets>
        <for.p in projets>
            <container.projet>
                <if.p.actif == true><button.projet-nom.on#projet-{{p.i}}>{{p.nom}}<!button><!if>
                <else><button.projet-nom#projet-{{p.i}}>{{p.nom}}<!button><!else>
                <if.p.env == true><text.projet-env>azure<!text><!if>
                <else><button.projet-x#delier-{{p.i}}>×<!button><!else>
            <!container>
        <!for>
        <!container>
        <!-- Composants : stockages > dossiers (imbriques) > composants. -->
        <container.cote-entete>
            <text.cote-titre.pousse>COMPOSANTS<!text>
            <button.cote-plus#comp-nouveau-stockage>+ Stockage<!button>
        <!container>
        <container.arbre>
        <if.a_composants_arbre == false>
            <text.cote-aide>Créez un stockage pour y ranger vos modèles de tests, dans des dossiers.<!text>
        <!if>
        <for.c in comp_arbre>
            <container.arb-ligne.arb-p{{c.prof}}>
                <if.c.a_enfants == true>
                    <if.c.plie == true><button.arb-pli#plier-{{c.id}}>+<!button><!if>
                    <else><button.arb-pli#plier-{{c.id}}>–<!button><!else>
                <!if>
                <else><container.arb-pli-vide><!container><!else>
                <container.arb-ic.{{c.genre}}><!container>
                <if.c.on == true><button.arb-nom.on#noeud-{{c.id}}>{{c.nom}}<!button><!if>
                <else><button.arb-nom#noeud-{{c.id}}>{{c.nom}}<!button><!else>
            <!container>
        <!for>
        <!container>

        <text.cote-titre.espace>RELIER UN PROJET<!text>
        <input.cote-champ#chemin placeholder="~/Dev/mon-projet" value="{{saisie_chemin}}"/>
        <button.cote-bouton#lier>Relier<!button>
        <text.cote-aide>Le dossier d'un projet (Cargo.toml). Hors de ~/Dev, ajoutez-le aux [permissions] de app.azure.<!text>
    <!container>

    <container.principal>
        <if.onglet == "composants"><include src="parts/composants.rsh"/><!if>
        <elseif.a_projet == false>
            <container.vide>
                <title2.vide-titre>Aucun projet<!title2>
                <text.vide-texte>Reliez un projet à gauche pour voir ses tests.<!text>
            <!container>
        <!elseif>
        <elseif.onglet == "tests"><include src="parts/tests.rsh"/><!elseif>
        <else><include src="parts/atelier.rsh"/><!else>

        <if.console == true>
            <container.console>
                <container.console-tete>
                    <text.console-titre>Sortie · {{exec_titre}}<!text>
                    <button.petit#console>Fermer<!button>
                <!container>
                <container.console-lignes>
                    <for.l in console_lignes><text.console-ligne>{{l}}<!text><!for>
                <!container>
            <!container>
        <!if>
    <!container>
<!container>

<!container>
