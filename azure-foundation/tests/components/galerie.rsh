<!-- Galerie : tous les composants d'Azure sur une page. -->
<container.galerie>
<navbar titre="Azure UI">
    <menu-item actif="true" id="nav-accueil">Accueil<!menu-item>
    <menu-item id="nav-docs">Docs<!menu-item>
    <spacer/>
    <avatar nom="Ada Lovelace"/>
<!navbar>

<page titre="Composants" description="Tout ce qu'une page rsH peut utiliser, sans rien ecrire en Rust.">

<breadcrumb items="Accueil, Composants, Galerie"/>

<section titre="Formulaire">
<grid cols="2">
    <form>
        <field label="Adresse e-mail" aide="Jamais partagee.">
            <input#email placeholder="vous@exemple.fr"/>
        <!field>
        <field label="Mot de passe" erreur="8 caracteres minimum">
            <password#mdp value="secret"/>
        <!field>
        <field label="Quantite"><number#quantite value="3"/><!field>
        <field label="Pays">
            <select#pays value="be">
                <option value="fr">France<!option>
                <option value="be">Belgique<!option>
                <option value="ch">Suisse<!option>
            <!select>
        <!field>
    <!form>
    <form>
        <checkbox#cgu checked="true">J'accepte les conditions<!checkbox>
        <checkbox#news>Recevoir la lettre<!checkbox>
        <row>
            <radio name="taille" value="S">S<!radio>
            <radio name="taille" value="M" checked="true">M<!radio>
            <radio name="taille" value="L">L<!radio>
        <!row>
        <switch#sombre checked="true" label="Theme sombre"/>
        <switch#notif label="Notifications"/>
        <slider#volume value="60"/>
        <segmented#vue options="Jour, Semaine, Mois" value="Semaine"/>
        <rating#note value="4" label="4 / 5"/>
    <!form>
<!grid>
<row>
    <btn.primary#envoyer>Envoyer<!btn>
    <btn#brouillon>Brouillon<!btn>
    <btn.danger#supprimer>Supprimer<!btn>
    <btn.ghost#annuler>Annuler<!btn>
    <link#aide>Besoin d'aide ?<!link>
<!row>
<!section>

<section titre="Indicateurs">
<grid cols="4">
    <stat label="Utilisateurs" valeur="12 480" detail="+8 % ce mois" tendance="hausse"/>
    <stat label="Ventes" valeur="3 214" detail="-2 %" tendance="baisse"/>
    <stat label="Panier moyen" valeur="46 EUR"/>
    <stat label="Taux de retour" valeur="1,8 %" detail="stable"/>
<!grid>
<meter label="Stockage" texte="72 / 100 Go" value="72"/>
<progress value="35"/>
<!section>

<section titre="Etiquettes">
<row>
    <badge>Nouveau<!badge>
    <badge.succes>En ligne<!badge>
    <badge.attention>En attente<!badge>
    <badge.danger>Erreur<!badge>
    <badge.neutre>Brouillon<!badge>
    <tag>rust<!tag>
    <tag>wayland<!tag>
    <chip id="retirer-rsh">rsH<!chip>
    <kbd>Ctrl<!kbd>
    <kbd>S<!kbd>
<!row>
<!section>

<section titre="Alertes">
<alert titre="Information">Une nouvelle version est disponible.<!alert>
<alert.succes titre="Enregistre">Vos modifications ont ete sauvegardees.<!alert>
<alert.attention titre="Attention">Votre abonnement expire dans 3 jours.<!alert>
<alert.danger titre="Erreur">Impossible de joindre le serveur.<!alert>
<toast.succes titre="Copie">Le lien est dans le presse-papiers.<!toast>
<!section>

<section titre="Navigation">
<tabs id="onglets" items="General, Securite, Notifications" actif="Securite"/>
<steps items="Panier, Livraison, Paiement, Confirmation" courant="3"/>
<pagination id="page" pages="6" page="2"/>
<!section>

<section titre="Contenu">
<grid cols="3">
    <card titre="Carte simple" description="Avec une description" pied="Mise a jour il y a 2 min">
        <text>Le contenu passe entre les balises.<!text>
    <!card>
    <fiche nom="Grace Hopper" role="Amirale" ville="Arlington"/>
    <empty titre="Aucun message" description="Les messages recus apparaitront ici.">
        <btn.small#ecrire>Ecrire<!btn>
    <!empty>
<!grid>
<list>
    <list-item titre="Facture 2024-118" description="Payee le 12 mars"><badge.succes>Payee<!badge><!list-item>
    <list-item titre="Facture 2024-119" description="Echeance le 30 mars"><badge.attention>En attente<!badge><!list-item>
<!list>
<table>
    <tr><th>Nom<!th><th>Role<!th><th>Statut<!th><!tr>
    <tr><td>Ada<!td><td>Admin<!td><td>Active<!td><!tr>
    <tr><td>Alan<!td><td>Lecteur<!td><td>Invite<!td><!tr>
<!table>
<accordion titre="Comment ca marche ?" ouvert="true" id="faq-1">
    <text>Chaque composant est un fichier rsH : on peut en creer d'autres dans components/.<!text>
<!accordion>
<accordion titre="Et les styles ?" id="faq-2"><text>Invisible tant que ferme.<!text><!accordion>
<timeline>
    <timeline-item titre="Commande passee" date="lundi"><text>3 articles<!text><!timeline-item>
    <timeline-item titre="Expediee" date="mardi"><text>Colis 8XJ21<!text><!timeline-item>
<!timeline>
<quote auteur="Grace Hopper">Le plus dangereux : on a toujours fait comme ca.<!quote>
<code>let app = azure_app!()?;<!code>
<info terme="Version" valeur="1.4.2"/>
<info terme="Licence" valeur="MIT"/>
<skeleton/>
<include src="parts/pied.rsh"/>
<!section>

<section titre="Pages">
<hero titre="Construisez vite" description="Des composants prets a l'emploi, stylables en rsC.">
    <btn.primary#commencer>Commencer<!btn>
    <btn#docs>Documentation<!btn>
<!hero>
<grid cols="3">
    <feature titre="Rapide" description="Rendu logiciel optimise."/>
    <feature titre="Sur" description="Stockage chiffre par app."/>
    <feature titre="Connecte" description="Flux et appels entre apps."/>
<!grid>
<grid cols="2">
    <price nom="Essentiel" prix="0 EUR" periode="/ mois" items="1 app, Stockage local"><btn#essentiel>Choisir<!btn><!price>
    <price nom="Pro" prix="9 EUR" periode="/ mois" items="Apps illimitees, Partage, Support"><btn.primary#pro>Choisir<!btn><!price>
<!grid>
<inconnu/>
<!section>
<!page>
<footer><text>Azure UI - galerie<!text><!footer>
<!container>
