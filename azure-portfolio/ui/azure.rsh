<!-- Azure : l'étude du projet, en chapitres. Chiffres relevés dans le code
     (septembre 2026). Les liens ouvrent Azure Docs (#doc-<page>) ou une
     app (#ouvrir-<app>) : voir src/lib.rs. -->
<container.app>
<include src="parts/menu.rsh"/>
<container.defile>
<container.page>
    <container.ouverture>
        <text.rubrique>ÉTUDE DE PROJET<!text>
        <text.affiche-2>Azure<!text>
        <text.chapeau>Un environnement d'applications complet pour Linux, écrit en Rust et sans framework. Pas de GTK, pas de Qt, pas de navigateur : tout est fait maison, de l'ouverture de la fenêtre jusqu'au stockage chiffré.<!text>
        <container.liens>
            <button.lien#ouvrir-docs>Ouvrir la documentation<!button>
            <button.lien#doc-vie-app>La vie d'une app, pas à pas<!button>
        <!container>
    <!container>

    <container.travail>
        <container.image.img-docs-rsc><!container>
        <text.legende>Azure Docs. Le texte, les tableaux, le code coloré et l'aperçu en direct sont dessinés par le moteur d'Azure.<!text>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>EN CHIFFRES<!text>
        <container.bloc-corps>
            <container.chiffres>
                <container.chiffre-bloc.premier><text.chiffre>56 000<!text><text.note>lignes de Rust<!text><!container>
                <container.chiffre-bloc><text.chiffre>670<!text><text.note>tests automatisés<!text><!container>
                <container.chiffre-bloc><text.chiffre>2<!text><text.note>dépendances : libc et ttf-parser<!text><!container>
            <!container>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>POURQUOI<!text>
        <container.bloc-corps>
            <text.grand-texte>Comprendre de bout en bout comment fonctionne un système d'applications : afficher une fenêtre, calculer une mise en page, faire parler des programmes entre eux, garder des données en sécurité. En le construisant soi-même, sans boîte noire.<!text>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>ARCHITECTURE<!text>
        <container.bloc-corps>
            <text.texte>Cinq couches. Une app ne dépend que d'azure-foundation, qui lui donne tout le reste ; les daemons tournent à part et parlent aux apps par des sockets unix.<!text>
            <container>
                <container.couche><text.couche-nom>Apps<!text><text.couche-contenu>Dashboard, Docs, Note, ce portfolio<!text><!container>
                <container.couche><text.couche-nom>Foundation<!text><text.couche-contenu>rsH et rsC, mise en page, composants, événements<!text><!container>
                <container.couche><text.couche-nom>Engine<!text><text.couche-contenu>Wayland, rendu logiciel, texte, images<!text><!container>
                <container.couche><text.couche-nom>Daemons<!text><text.couche-contenu>provider, rooter, stockage, service, manager<!text><!container>
                <container.couche><text.couche-nom>Core<!text><text.couche-contenu>format binaire, cryptographie, sécurité, identité<!text><!container>
            <!container>
            <button.lien#doc-ensemble>Tout le fonctionnement dans la documentation<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>AFFICHER<!text>
        <container.bloc-corps>
            <text.titre>Parler Wayland directement, calculer chaque pixel.<!text>
            <text.texte>Azure ouvre ses fenêtres en parlant le protocole du compositeur sur son socket, sans bibliothèque. Le rendu est logiciel : dégradés, ombres, coins arrondis, texte à partir de vraies polices, images PNG décodées à la main. Le clavier suit la carte du compositeur, AZERTY et touches mortes comprises.<!text>
            <button.lien#doc-engine>azure-engine dans la documentation<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>DÉCRIRE<!text>
        <container.bloc-corps>
            <text.titre>Deux langages pour les interfaces.<!text>
            <text.texte>rsH décrit une page, comme HTML, avec des conditions, des boucles et des inclusions. rsC la met en forme, comme CSS : cascade, flex, grid, survol, transitions. Les pages sont relues à chaud ; celle-ci en est une.<!text>
            <button.lien#doc-foundation>azure-foundation dans la documentation<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>GARDER<!text>
        <container.bloc-corps>
            <text.titre>Des données chiffrées, et un SQL maison.<!text>
            <text.texte>Un seul processus écrit les données des apps. Chacune a son espace, chiffré avec ChaCha20-Poly1305, aux noms de fichiers illisibles. RsS, le SQL d'Azure, gère tables, jointures, index et transactions avec journal.<!text>
            <button.lien#doc-stockage>azure-stockage dans la documentation<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>RELIER<!text>
        <container.bloc-corps>
            <text.titre>Des apps qui se parlent, sous contrôle.<!text>
            <text.texte>Le routeur transporte messages et navigation d'une app à l'autre, et garde le courrier des apps fermées. Les flux diffusent un état en temps réel ; les appels attendent une réponse. Le manager décide qui a le droit de quoi. Un clic sur les liens de cette page passe d'ailleurs par là : le portfolio demande au manager de lancer l'app, et le routeur lui porte la page à ouvrir.<!text>
            <container.liens>
                <button.lien#doc-rooter>Le routeur<!button>
                <button.lien#doc-manager>Le manager<!button>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.paire>
            <container.paire-col>
                <container.image.courte.img-dashboard><!container>
                <text.legende>Le tableau de bord : les apps, leurs droits, leurs journaux.<!text>
                <button.lien#ouvrir-dashboard>Ouvrir le tableau de bord<!button>
            <!container>
            <container.paire-col>
                <container.image.courte.img-note><!container>
                <text.legende>Azure Note, qui cherche dans la documentation en appelant Azure Docs.<!text>
                <button.lien#ouvrir-note>Ouvrir Azure Note<!button>
            <!container>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>PROTÉGER<!text>
        <container.bloc-corps>
            <text.titre>Toutes les apps tournent sous le même utilisateur ; Azure les sépare lui-même.<!text>
            <text.texte>Un daemon ne croit jamais ce qu'une app annonce : il demande au noyau qui est au bout du socket, puis vérifie l'empreinte SHA-256 de son exécutable. Chaque app est enfermée par Landlock et des espaces de noms. La cryptographie est écrite à la main et vérifiée contre les normes.<!text>
            <button.lien#doc-core>Identité, cryptographie et sécurité<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>OUTILLER<!text>
        <container.bloc-corps>
            <text.texte>Une commande pour tout : azure new crée une app, azure build la compile, azure install l'installe, azure run la lance. Le tableau de bord fait la même chose depuis son terminal, ou d'un clic sur la page d'une app.<!text>
            <button.lien#doc-cli>La commande azure<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>CE QUE J'EN RETIENS<!text>
        <container.bloc-corps>
            <text.grand-texte>Azure a d'abord été un test pour moi-même : savoir si j'étais capable de mener seul un projet de cette taille, du premier pixel jusqu'au dernier daemon.<!text>
            <text.texte>Il m'a appris la rigueur. Quand on écrit soi-même le protocole d'affichage, la cryptographie ou le stockage, aucune bibliothèque ne rattrape une erreur : chaque partie doit être comprise, testée, puis documentée. Il m'a appris aussi la discipline : avancer chaque jour, écrire les tests avant de passer à la suite, reprendre ce qui ne tient pas plutôt que de l'empiler.<!text>
            <text.texte>C'est enfin une validation de mes compétences. Chaque notion vue en cours, je l'ai mise à l'épreuve ici, dans un système qui tourne vraiment : ce portfolio en est la preuve.<!text>
        <!container>
    <!container>
<!container>
<include src="parts/pied.rsh"/>
<!container>
<!container>
