<!-- Présentation : à remplir à la main (remplacer chaque [ ... ]).
     Une image de projet : ui/images/<nom>.png, puis une classe
     .img-<nom> { background-image: url(ui/images/<nom>.png); } dans app.rsc. -->
<container.app>
<include src="parts/menu.rsh"/>
<container.defile>
<container.page>
    <container.ouverture>
        <text.rubrique>PORTFOLIO — 2026<!text>
        <text.affiche>Yoann FAYOLLE construit des logiciels de bout en bout, du pixel jusqu'au protocole.<!text>
        <text.chapeau> Étudiant en BTS SIO option SLAM à Paris Je suis en alternance  chez BAOBA à Plaisir <!text>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Azure<!text>
            <text.travail-annee>[2025 — 2026]<!text>
        <!container>
        <container.image.img-docs><!container>
        <text.legende>Azure Docs, l'une des apps d'Azure. Chaque pixel est calculé par le moteur d'Azure.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>RÔLE<!text>
                <text.meta>[Seul développeur]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>Rust, Wayland, Linux<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Un environnement d'applications complet pour Linux, écrit sans framework : fenêtres, rendu, langages d'interface, stockage chiffré, communication entre apps.<!text>
                <button.lien#nav-azure>Lire l'étude du projet<!button>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>AutomatImport<!text>
            <text.travail-annee>2026<!text>
        <!container>
        <container.image.courte.AutomatImport><!container>
        <text.legende>AutomatImport est un outil qui a pour but de faire de l'import de masse de données.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>RÔLE<!text>
                <text.meta>[Seul assisté des équipe data]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>[Python, CSV]<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Il prend un csv en entrée le parse puis récupére les data pour faire des insertion sql direcement en CLI.<!text>
                <button.lien#nav-projets>Tous les projets<!button>
            <!container>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>À PROPOS<!text>
        <container.bloc-corps>
            <text.grand-texte>Passionné d'informatique d'informatique depuis petit mon premier pas dans l'informatique ce fait de par mon papa réparteur informatique. Au lycée je me lance dans un bac Pro système numérique option Risc. Puis lors d'un stage dans le mileur de la proggramtion je décide de m'orienté en bts SIO option SLAM a ipssi paris afin de parfaire mon savoir du monde du dev.<!text>
            <text.texte>Mon experience de develloper commence par le dev web (HTML, CSS, PHP, JS, SQL) puis continue son chemain dans le merveilleux monde du dev logiciel (C#, Rust, Python)<!text>
            <button.lien#nav-etudes>Mon parcours<!button>
        <!container>
    <!container>

    <container.bloc>
        <text.bloc-etiquette>COMPÉTENCES<!text>
        <container.bloc-corps>
            <container>
                <container.couche><text.couche-nom>Web<!text><text.couche-contenu>HTML, CSS, JavaScript, PHP, Laravel<!text><!container>
                <container.couche><text.couche-nom>Logiciel<!text><text.couche-contenu>C#, .NET MAUI, Rust, Python<!text><!container>
                <container.couche><text.couche-nom>Données<!text><text.couche-contenu>SQL, MySQL, import de données en masse<!text><!container>
                <container.couche><text.couche-nom>Tests<!text><text.couche-contenu>Tests unitaires et d'intégration avec NUnit, tests d'interface avec Appium<!text><!container>
                <container.couche><text.couche-nom>Systèmes<!text><text.couche-contenu>Linux, Wayland, réseaux (switchs, VLAN, fibre), matériel<!text><!container>
            <!container>
        <!container>
    <!container>

    <container.contact>
        <text.rubrique>ÉCRIRE<!text>
        <text.contact-mail>yoann.fayolle@ecole-ipssi.net<!text>
        <text.note>GitHub : https://github.com/AynnoIpssi<!text>
    <!container>
<!container>
<include src="parts/pied.rsh"/>
<!container>
<!container>
