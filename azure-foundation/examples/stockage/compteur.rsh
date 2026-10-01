<!-- Page de la demo stockage_demo : {{...}} est remplace par la valeur lue
     dans azure-stockage avant chaque affichage (rsH n'a pas encore
     d'interpolation, voir PROBLEMES.md #10). -->
<container.app>
    <text.badge>azure-stockage · app {{app}}<!text>
    <text.title>Compteur qui se souvient<!text>
    <text.body>Ferme la fenêtre et relance la démo : le compte est gardé, chiffré, par le daemon de stockage.<!text>
    <container.chiffre>
        <text.valeur>{{clics}}<!text>
        <text.legende>clics<!text>
    <!container>
    <container.actions>
        <button.plus#plus>+1<!button>
        <button.zero#zero>Remettre à zéro<!button>
    <!container>
    <text.info>Démo ouverte {{ouvertures}} fois · dernier clic : {{dernier}}<!text>
<!container>
