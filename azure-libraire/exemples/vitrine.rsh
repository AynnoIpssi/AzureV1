<!-- Vitrine : une app complete faite uniquement de modules d'interface.
     Dessinee dans chaque theme par azure-foundation/tests/libraire_themes.rs. -->
<app>
    <sidebar titre="Vitrine">
        <sidebar-item id="nav-accueil" actif="true">Accueil<!sidebar-item>
        <sidebar-item id="nav-projets">Projets<!sidebar-item>
        <sidebar-group titre="FICHIERS">
            <tree-item id="t-src" niveau="0" ouvert="true">src<!tree-item>
            <tree-item id="t-main" niveau="1" actif="true">main.rs<!tree-item>
            <tree-item id="t-ui" niveau="1" ouvert="false">ui<!tree-item>
        <!sidebar-group>
    <!sidebar>
    <main>
        <toolbar titre="Tableau de bord">
            <search-bar id="recherche" placeholder="Rechercher..." bouton="Chercher"/>
            <spacer/>
            <btn-group><btn.small id="vue-jour">Jour<!btn><btn.small id="vue-mois">Mois<!btn><!btn-group>
            <icon-btn id="plus">+<!icon-btn>
        <!toolbar>
        <page>
            <header titre="Bonjour {{qui}}" description="Tout ce qui suit est un module de la librairie.">
                <btn id="exporter">Exporter<!btn>
                <btn.primary id="nouveau">Nouveau<!btn>
            <!header>
            <grid cols="3">
                <stat label="Visites" valeur="12 480" detail="+8 % ce mois" tendance="hausse"/>
                <stat label="Erreurs" valeur="3" detail="-2 cette semaine" tendance="baisse"/>
                <card titre="Equipe" description="Qui travaille dessus">
                    <media nom="Ada Lovelace" titre="Ada Lovelace" description="Analyste"><count.accent>4<!count><!media>
                    <status.succes>En ligne<!status>
                <!card>
            <!grid>
            <alert.attention titre="Attention">Le disque est presque plein.<!alert>
            <grid cols="2">
                <panel titre="REGLAGES">
                    <setting titre="Notifications" description="Un message a chaque fin de tache"><switch#notifs checked="true"/><!setting>
                    <setting titre="Langue"><select#langue options="Francais, English"/><!setting>
                    <shortcut label="Rechercher" touches="Ctrl, K"/>
                <!panel>
                <fieldset titre="Compte" description="Vos informations">
                    <field label="Nom" aide="Affiche dans l'app."><input#nom value="Ada"/><!field>
                    <label>Role<!label>
                    <segmented#role options="Lecture, Ecriture"/>
                    <help.erreur>Choix obligatoire<!help>
                    <actions><btn id="annuler">Annuler<!btn><btn.primary id="valider">Enregistrer<!btn><!actions>
                <!fieldset>
            <!grid>
            <caption>JOURNAL<!caption>
            <lead>Les dernieres operations.<!lead>
            <table>
                <tr><th>Fichier<!th><th>Etat<!th><!tr>
                <tr><td><text>main.rs<!text><!td><td><row><badge.succes>Compile<!badge><!row><!td><!tr>
            <!table>
            <muted>Mis a jour a l'instant.<!muted>
            <tabs id="onglets" items="General, Securite" actif="General"/>
            <box.borde><code>let app = azure_app!()?;<!code><!box>
        <!page>
        <statusbar><text>Pret<!text><spacer/><text>3 themes<!text><!statusbar>
    <!main>
<!app>
<confirm.danger id="suppr" titre="Supprimer le projet ?" ouvert="{{question}}" oui="Supprimer">Cette action est definitive.<!confirm>
