<!-- Test de l'interpolation : {{...}}, boucle avec variable, objets, ids de bouton. -->
<container.page>
    <title>{{titre}} ({{total}} apps)<!title>
    <for.app in apps>
        <container.ligne>
            <text>{{app_index}}. {{ app.nom }}<!text>
            <if.app.actif == true><text>actif (pid {{app.pid}})<!text><!if><else><text>arrete<!text><!else>
            <button.ouvrir#ouvrir-{{app.nom}}>Ouvrir {{app.nom}}<!button>
        <!container>
    <!for>
    <text>Inconnu : [{{rien}}] - fin<!text>
<!container>
