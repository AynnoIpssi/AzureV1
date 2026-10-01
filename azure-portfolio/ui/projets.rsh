<!-- Projets : un projet = un bloc <container.travail>. Pour une image :
     ui/images/<nom>.png et une classe .img-<nom> dans app.rsc (à la place
     de .vide). -->
<container.app>
<include src="parts/menu.rsh"/>
<container.defile>
<container.page>
    <container.ouverture>
        <text.rubrique>PROJETS<!text>
        <text.affiche>Ce que j'ai construit, et ce que chaque projet m'a appris.<!text>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Azure<!text>
            <text.travail-annee>[2025 — 2026]<!text>
        <!container>
        <container.image.img-dashboard><!container>
        <text.legende>Le tableau de bord d'Azure : les apps installées, leurs droits, qui parle à qui.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>[Projet personnel]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>Rust, sans framework<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Un environnement d'applications pour Linux, du protocole d'affichage jusqu'au stockage chiffré. Ce portfolio en est une app.<!text>
                <button.lien#nav-azure>Lire l'étude du projet<!button>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Tests MAUI<!text>
            <text.travail-annee>2026<!text>
        <!container>
        <container.image.img-tests-maui><!container>
        <text.legende>Les trois niveaux de tests dans l'explorateur NUnit ; à droite, Appium pilote l'application comme le ferait un utilisateur.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>Projet scolaire, BTS SIO<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>C#, .NET MAUI, NUnit, Appium, SQL<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>L'automatisation des tests d'une application .NET MAUI adossée à une base SQL : tests unitaires, tests d'intégration, tests d'interface.<!text>
                <text.texte>Avec NUnit, les tests unitaires vérifient la logique de l'application, isolée du reste. Les tests d'intégration la font travailler contre la vraie base SQL : on enregistre, on relit, on vérifie ce qui a été écrit. Appium, enfin, pilote l'application elle-même : il remplit les champs, clique sur les boutons et contrôle l'écran suivant, comme le ferait un utilisateur. Toute la suite se relance en une commande : une régression se voit avant d'arriver chez l'utilisateur.<!text>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>AutomatImport<!text>
            <text.travail-annee>2026<!text>
        <!container>
        <container.image.courte.AutomatImport><!container>
        <text.legende>Import d'un fichier CSV dans une base MySQL, en ligne de commande.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>[Alternance / projet]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>Python, CSV, MySQL<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Un outil en ligne de commande pour importer des données en masse : il lit un CSV, contrôle chaque ligne et l'insère en base.<!text>
                <text.texte>La réalisation de ce projet était un challenge du a une deadline sérer. L'outile a permit de gagné du temp pour les équipes.<!text>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Scratch Lite<!text>
            <text.travail-annee>2025<!text>
        <!container>
        <container.image.img-scratch><!container>
        <text.legende>L'éditeur : les blocs se prennent dans la palette et s'emboîtent ; le lutin exécute le script sur la scène.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>[Projet personnel]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>HTML, CSS, JavaScript<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Ma version de Scratch, refaite en HTML, CSS et JavaScript : une porte d'entrée pour apprendre la logique de la programmation.<!text>
                <text.texte>On assemble des blocs au lieu d'écrire du code : séquences, boucles, conditions, variables. Sans syntaxe à retenir, il ne reste que la logique, ce qui en fait un premier pas avant un vrai langage. Côté technique : le glisser-déposer et l'emboîtement des blocs, puis un petit interpréteur qui parcourt le script bloc par bloc et fait bouger le lutin à chaque image.<!text>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Pixel Shop<!text>
            <text.travail-annee>2025-2026<!text>
        <!container>
        <container.image.img-boutique><!container>
        <text.legende>Le catalogue : filtres par catégorie et par prix, ajout au panier depuis chaque fiche.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>[Contexte]<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>Laravel, MySQL, HTML, CSS<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Une boutique en ligne complète, habillée dans un style pixel art.<!text>
                <text.texte>Catalogue, panier, comptes clients et commandes, avec Laravel et une base MySQL : les routes et les contrôleurs côté serveur, les modèles Eloquent pour les produits et les commandes, les vues Blade pour les pages. Tout le style pixel est fait en CSS : bordures épaisses, ombres franches, police à chasse fixe.<!text>
            <!container>
        <!container>
    <!container>

    <container.travail>
        <container.travail-tete>
            <text.travail-titre>Échecs<!text>
            <text.travail-annee>2023<!text>
        <!container>
        <container.image.img-echecs><!container>
        <text.legende>Une partie dans la console : le plateau en texte, les coups tapés au clavier, les coups interdits refusés.<!text>
        <container.travail-texte>
            <container.travail-texte-gauche>
                <text.meta-nom>CONTEXTE<!text>
                <text.meta>Stage de bac pro<!text>
                <text.meta-nom>AVEC<!text>
                <text.meta>C#, console<!text>
            <!container>
            <container.travail-texte-droite>
                <text.grand-texte>Mon premier programme complet : un jeu d'échecs en C#, en ligne de commande.<!text>
                <text.texte>Un projet basique, sans interface graphique : le plateau s'affiche en texte, les joueurs tapent leurs coups au clavier, et le programme vérifie que chaque déplacement respecte les règles de la pièce. C'est ce projet qui m'a fait passer du matériel au logiciel.<!text>
            <!container>
        <!container>
    <!container>

<!container>
<include src="parts/pied.rsh"/>
<!container>
<!container>
